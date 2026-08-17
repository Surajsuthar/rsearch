use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use rsearch::app::{App, Mode};
use rsearch::index::Index;
use rsearch::{storage, ui};

#[derive(Parser)]
#[command(name = "rsearch", about = "Local full-text search over a directory (BM25, TUI, vim-style)")]
struct Cli {
    /// Directory to index and search (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Force a full reindex, ignoring any existing on-disk index
    #[arg(short, long)]
    rebuild: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.path.canonicalize()?;

    let existing = if cli.rebuild {
        Index::new()
    } else {
        storage::load(&root).unwrap_or_default()
    };

    println!("Indexing {}...", root.display());
    let (index, stats) = storage::build_or_update(&root, existing, |done, total| {
        if total > 0 && (done % 200 == 0 || done == total) {
            use std::io::Write;
            print!("\r  {done}/{total} files scanned");
            let _ = io::stdout().flush();
        }
    })?;
    println!(
        "\rIndex ready: {} docs ({} added, {} updated, {} removed) in {} ms",
        stats.total_docs, stats.added, stats.updated, stats.removed, stats.elapsed_ms
    );
    storage::save(&root, &index)?;

    run_tui(index, root)
}

fn run_tui(index: Index, root: PathBuf) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(index);
    let res = event_loop(&mut terminal, &mut app, &root);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    root: &PathBuf,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match app.mode {
                    Mode::Normal => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('/') => {
                            app.mode = Mode::Search;
                            app.query.clear();
                        }
                        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
                        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
                        KeyCode::Char('r') => {
                            let existing = std::mem::take(&mut app.index);
                            let (new_index, stats) = storage::build_or_update(root, existing, |_, _| {})?;
                            storage::save(root, &new_index)?;
                            app.status = format!(
                                "reindexed: {} added, {} updated, {} removed ({} ms)",
                                stats.added, stats.updated, stats.removed, stats.elapsed_ms
                            );
                            app.index = new_index;
                            if !app.query.is_empty() {
                                app.run_search();
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(path) = app.selected_path().map(|s| s.to_string()) {
                                open_in_editor(terminal, &path)?;
                            }
                        }
                        _ => {}
                    },
                    Mode::Search => match key.code {
                        KeyCode::Esc => app.mode = Mode::Normal,
                        KeyCode::Enter => {
                            app.mode = Mode::Normal;
                            app.run_search();
                        }
                        KeyCode::Backspace => {
                            app.query.pop();
                        }
                        KeyCode::Char(c) => app.query.push(c),
                        _ => {}
                    },
                }
            }
        }
    }
}

fn open_in_editor(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, path: &str) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    let status = std::process::Command::new(editor).arg(path).status();

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    status?;
    Ok(())
}
