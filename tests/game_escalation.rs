// @spec-tags: core,game,escalation
// @invariants: escalation_stage returns Danger if any occupied cell exists at row < DANGER_THRESHOLD_ROW (4), otherwise Normal
// @build: 33

use rhythm_grid::game::{escalation_stage, EscalationStage, DANGER_THRESHOLD_ROW};
use rhythm_grid::grid::{CellState, Grid};

#[test]
fn danger_threshold_row_constant_is_4() {
    assert_eq!(DANGER_THRESHOLD_ROW, 4usize);
}

#[test]
fn escalation_stage_empty_grid_is_normal() {
    let grid = Grid::new();
    assert_eq!(escalation_stage(&grid), EscalationStage::Normal);
}

#[test]
fn escalation_stage_occupied_at_or_below_threshold_is_normal() {
    // Row == DANGER_THRESHOLD_ROW (4) is NOT above threshold; stage is Normal.
    let mut grid = Grid::new();
    grid.cells[DANGER_THRESHOLD_ROW][0] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Normal);
}

#[test]
fn escalation_stage_occupied_well_below_grid_top_is_normal() {
    // Row 10 is safely below threshold; stage is Normal.
    let mut grid = Grid::new();
    grid.cells[10][3] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Normal);
}

#[test]
fn escalation_stage_occupied_above_threshold_is_danger() {
    // Row 3 < DANGER_THRESHOLD_ROW(4) → Danger.
    let mut grid = Grid::new();
    grid.cells[3][0] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Danger);
}

#[test]
fn escalation_stage_occupied_at_row_zero_is_danger() {
    // Row 0 is the topmost row; any occupied cell there triggers Danger.
    let mut grid = Grid::new();
    grid.cells[0][0] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Danger);
}

#[test]
fn escalation_stage_danger_overrides_normal_cells() {
    // Even when rows >= 4 also have occupied cells, one cell in row < 4 triggers Danger.
    let mut grid = Grid::new();
    grid.cells[5][0] = CellState::Occupied(1);
    grid.cells[8][3] = CellState::Occupied(1);
    grid.cells[2][5] = CellState::Occupied(1); // row 2 < 4 → Danger
    assert_eq!(escalation_stage(&grid), EscalationStage::Danger);
}

#[test]
fn escalation_stage_row_1_occupied_is_danger() {
    let mut grid = Grid::new();
    grid.cells[1][9] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Danger);
}

#[test]
fn escalation_stage_row_2_occupied_is_danger() {
    let mut grid = Grid::new();
    grid.cells[2][0] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Danger);
}

#[test]
fn escalation_stage_row_3_occupied_is_danger() {
    let mut grid = Grid::new();
    grid.cells[3][9] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Danger);
}

#[test]
fn escalation_stage_only_bottom_row_occupied_is_normal() {
    // Row 19 (bottom) is well below threshold; stage is Normal.
    let mut grid = Grid::new();
    grid.cells[19][5] = CellState::Occupied(1);
    assert_eq!(escalation_stage(&grid), EscalationStage::Normal);
}

#[test]
fn escalation_stage_enum_has_debug_and_partial_eq() {
    // Both derived traits must be present.
    let s = format!("{:?}", EscalationStage::Normal);
    assert!(!s.is_empty());
    assert_eq!(EscalationStage::Danger, EscalationStage::Danger);
    assert_ne!(EscalationStage::Normal, EscalationStage::Danger);
}
