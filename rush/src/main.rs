use std::error::Error;
use terminal::Terminal;

mod terminal;

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = Terminal::new()?;
    terminal.run()
}