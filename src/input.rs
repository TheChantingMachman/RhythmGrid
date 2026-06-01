// Input mapping — pipeline-owned testable logic.
// Wiring to winit events lives in main.rs (co-authored).

/// Project-local KeyCode enum — decoupled from winit so tests run without a windowing dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Left,
    Right,
    Down,
    Up,
    Z,
    X,
    Space,
    P,
    Escape,
    Enter,
    C,
    /// Catch-all for keys with no game-action binding.
    Other,
}

/// Actions the game logic can respond to, independent of input source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAction {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    RotateCW,
    RotateCCW,
    Hold,
    TogglePause,
    BackToMenu,
    StartGame,
}

const DAS_DEFAULT_MS: f64 = 150.0;
const ARR_DEFAULT_MS: f64 = 33.0;

/// Horizontal direction for auto-shift.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShiftDir {
    Left,
    Right,
}

/// Movement output from AutoShift::update.
///
/// Steps(n) — apply n horizontal moves this frame (n >= 1).
/// ChargeToWall — apply move_horizontal repeatedly until collision (ARR=0 instant charge).
#[derive(PartialEq, Debug)]
pub enum ShiftMove {
    Steps(u32),
    ChargeToWall,
}

/// DAS/ARR horizontal auto-shift timing state machine.
///
/// Feed key press/release events and a per-frame dt; consume the returned movement.
pub struct AutoShift {
    das_ms: f64,
    arr_ms: f64,
    active: Option<ShiftDir>,
    held_left: bool,
    held_right: bool,
    held_ms: f64,
    initial_tap_emitted: bool,
    arr_steps_emitted: u32,
    charge_latched: bool,
}

impl AutoShift {
    pub fn new(das_ms: f64, arr_ms: f64) -> Self {
        AutoShift {
            das_ms,
            arr_ms,
            active: None,
            held_left: false,
            held_right: false,
            held_ms: 0.0,
            initial_tap_emitted: false,
            arr_steps_emitted: 0,
            charge_latched: false,
        }
    }

    /// Register a direction key-down. Last-key-wins: a new direction restarts DAS from zero.
    pub fn press(&mut self, dir: ShiftDir) {
        match dir {
            ShiftDir::Left => self.held_left = true,
            ShiftDir::Right => self.held_right = true,
        }
        if self.active != Some(dir) {
            self.active = Some(dir);
            self.held_ms = 0.0;
            self.initial_tap_emitted = false;
            self.arr_steps_emitted = 0;
            self.charge_latched = false;
        }
    }

    /// Register a key-up. If the opposite direction is still held, control reverts to it.
    pub fn release(&mut self, dir: ShiftDir) {
        match dir {
            ShiftDir::Left => self.held_left = false,
            ShiftDir::Right => self.held_right = false,
        }
        if self.active == Some(dir) {
            let opposite_held = match dir {
                ShiftDir::Left => self.held_right,
                ShiftDir::Right => self.held_left,
            };
            if opposite_held {
                let opposite = match dir {
                    ShiftDir::Left => ShiftDir::Right,
                    ShiftDir::Right => ShiftDir::Left,
                };
                self.active = Some(opposite);
                self.held_ms = 0.0;
                self.initial_tap_emitted = false;
                self.arr_steps_emitted = 0;
                self.charge_latched = false;
            } else {
                self.active = None;
            }
        }
    }

    /// Advance internal time by dt_ms and return movement to apply this frame, or None.
    pub fn update(&mut self, dt_ms: f64) -> Option<(ShiftDir, ShiftMove)> {
        let dir = self.active?;
        self.held_ms += dt_ms;

        if self.arr_ms == 0.0 {
            if self.charge_latched {
                return None;
            }
            if self.das_ms == 0.0 {
                // Both zero: ChargeToWall on first update; initial tap subsumed.
                self.initial_tap_emitted = true;
                self.charge_latched = true;
                return Some((dir, ShiftMove::ChargeToWall));
            }
            // arr=0, das>0: emit initial tap before DAS, then ChargeToWall once DAS reached.
            if self.held_ms >= self.das_ms {
                self.charge_latched = true;
                return Some((dir, ShiftMove::ChargeToWall));
            }
            if !self.initial_tap_emitted {
                self.initial_tap_emitted = true;
                return Some((dir, ShiftMove::Steps(1)));
            }
            return None;
        }

        // Normal ARR case (arr_ms > 0)
        let mut total_steps = 0u32;

        if !self.initial_tap_emitted {
            self.initial_tap_emitted = true;
            total_steps += 1;
        }

        if self.held_ms >= self.das_ms {
            let arr_total = ((self.held_ms - self.das_ms) / self.arr_ms) as u32;
            let new_steps = arr_total.saturating_sub(self.arr_steps_emitted);
            self.arr_steps_emitted = arr_total;
            total_steps += new_steps;
        }

        if total_steps > 0 {
            Some((dir, ShiftMove::Steps(total_steps)))
        } else {
            None
        }
    }

    pub fn active_dir(&self) -> Option<ShiftDir> {
        self.active
    }

    /// Clear all held/active state, timers, and latches.
    pub fn reset(&mut self) {
        self.active = None;
        self.held_left = false;
        self.held_right = false;
        self.held_ms = 0.0;
        self.initial_tap_emitted = false;
        self.arr_steps_emitted = 0;
        self.charge_latched = false;
    }
}

impl Default for AutoShift {
    fn default() -> Self {
        AutoShift::new(DAS_DEFAULT_MS, ARR_DEFAULT_MS)
    }
}

/// Maps a key press to a game action, if one is bound.
pub fn map_key(key: KeyCode) -> Option<GameAction> {
    match key {
        KeyCode::Left    => Some(GameAction::MoveLeft),
        KeyCode::Right   => Some(GameAction::MoveRight),
        KeyCode::Down    => Some(GameAction::SoftDrop),
        KeyCode::Up      => Some(GameAction::RotateCW),
        KeyCode::X       => Some(GameAction::RotateCW),
        KeyCode::Z       => Some(GameAction::RotateCCW),
        KeyCode::Space   => Some(GameAction::HardDrop),
        KeyCode::C       => Some(GameAction::Hold),
        KeyCode::P       => Some(GameAction::TogglePause),
        KeyCode::Escape  => Some(GameAction::BackToMenu),
        KeyCode::Enter   => Some(GameAction::StartGame),
        _                => None,
    }
}
