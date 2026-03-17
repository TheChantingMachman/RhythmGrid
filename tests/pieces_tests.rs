// @spec-tags: core,pieces
// @invariants: TETROMINO_COUNT=7; CELLS_PER_PIECE=4; all 7 tetromino types (I,O,T,S,Z,J,L) exist; each has exactly 4 cells
// @build: 14

use rhythm_grid::pieces::{TetrominoType, TETROMINO_COUNT, CELLS_PER_PIECE};

#[test]
fn test_tetromino_count_constant_is_7() {
    assert_eq!(TETROMINO_COUNT, 7);
}

#[test]
fn test_cells_per_piece_constant_is_4() {
    assert_eq!(CELLS_PER_PIECE, 4);
}

#[test]
fn test_all_7_tetromino_types_exist() {
    let types = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    assert_eq!(types.len(), 7);
}

#[test]
fn test_tetromino_i_has_exactly_4_cells() {
    let cells = TetrominoType::I.cells();
    assert_eq!(cells.len(), CELLS_PER_PIECE);
}

#[test]
fn test_tetromino_o_has_exactly_4_cells() {
    let cells = TetrominoType::O.cells();
    assert_eq!(cells.len(), CELLS_PER_PIECE);
}

#[test]
fn test_tetromino_t_has_exactly_4_cells() {
    let cells = TetrominoType::T.cells();
    assert_eq!(cells.len(), CELLS_PER_PIECE);
}

#[test]
fn test_tetromino_s_has_exactly_4_cells() {
    let cells = TetrominoType::S.cells();
    assert_eq!(cells.len(), CELLS_PER_PIECE);
}

#[test]
fn test_tetromino_z_has_exactly_4_cells() {
    let cells = TetrominoType::Z.cells();
    assert_eq!(cells.len(), CELLS_PER_PIECE);
}

#[test]
fn test_tetromino_j_has_exactly_4_cells() {
    let cells = TetrominoType::J.cells();
    assert_eq!(cells.len(), CELLS_PER_PIECE);
}

#[test]
fn test_tetromino_l_has_exactly_4_cells() {
    let cells = TetrominoType::L.cells();
    assert_eq!(cells.len(), CELLS_PER_PIECE);
}

#[test]
fn test_all_tetromino_types_have_exactly_4_cells() {
    let types = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    for t in &types {
        assert_eq!(t.cells().len(), CELLS_PER_PIECE, "{:?} must have exactly 4 cells", t);
    }
}

#[test]
fn test_tetromino_cells_are_2d_coordinates() {
    // Each cell must be a tuple or struct with two integer components (col, row)
    let cells = TetrominoType::I.cells();
    for (col, row) in &cells {
        // coordinates are valid i32 values — just ensure they compile and are finite
        let _: i32 = *col;
        let _: i32 = *row;
    }
}

#[test]
fn test_tetromino_types_are_distinct() {
    // Verify each type produces a unique cell set relative to at least one other type
    let i_cells = TetrominoType::I.cells();
    let o_cells = TetrominoType::O.cells();
    assert_ne!(i_cells, o_cells, "I and O shapes must be distinct");
}

#[test]
fn test_tetromino_count_matches_enum_variant_count() {
    let all_types = [
        TetrominoType::I,
        TetrominoType::O,
        TetrominoType::T,
        TetrominoType::S,
        TetrominoType::Z,
        TetrominoType::J,
        TetrominoType::L,
    ];
    assert_eq!(all_types.len(), TETROMINO_COUNT as usize);
}
