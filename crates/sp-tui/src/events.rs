use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};

/// Terminal input events produced by the event loop.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    Key(KeyEvent),
    Tick,
}

/// Poll for the next event, blocking at most `tick_rate`.
pub fn next_event(tick_rate: Duration) -> std::io::Result<TuiEvent> {
    if event::poll(tick_rate)? {
        if let Event::Key(key) = event::read()? {
            return Ok(TuiEvent::Key(key));
        }
    }
    Ok(TuiEvent::Tick)
}
