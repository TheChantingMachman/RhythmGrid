// @spec-tags: render,draw,ui
// @invariants: board_quads emits quads for occupied cells, active piece (alpha=255), and ghost piece (alpha=80); ghost coinciding with active is skipped; next_piece_quads returns 4 quads from rotation-0 cells at correct pixel offsets with piece_color
// @build: 52

use rhythm_grid::game::ActivePiece;
use rhythm_grid::grid::{CellState, Grid};
use rhythm_grid::pieces::{TetrominoType, PIECE_CELLS};
use rhythm_grid::render::{board_quads, next_piece_quads, piece_color};

// ---- board_quads: empty grid, active piece at spawn ----

#[test]
fn board_quads_empty_grid_returns_8_quads_for_i_piece() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    // I-piece at row 0, empty grid → ghost at row 19 (no overlap) → 4 active + 4 ghost = 8.
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    assert_eq!(quads.len(), 8);
}

#[test]
fn board_quads_active_piece_cells_have_full_alpha() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    // Active piece quads are at py_y = row*30 = 0 (all I-piece cells have dr=0).
    let active_quads: Vec<_> = quads.iter().filter(|q| q.1 == 0).collect();
    assert_eq!(active_quads.len(), 4);
    for q in &active_quads {
        assert_eq!(q.4[3], 255, "Active piece quads must have alpha=255");
    }
}

#[test]
fn board_quads_ghost_piece_cells_have_alpha_80() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    // Ghost is at row 19 → py_y = 19*30 = 570.
    let ghost_quads: Vec<_> = quads.iter().filter(|q| q.1 == 570).collect();
    assert_eq!(ghost_quads.len(), 4);
    for q in &ghost_quads {
        assert_eq!(q.4[3], 80, "Ghost piece quads must have alpha=80");
    }
}

#[test]
fn board_quads_active_piece_correct_pixel_positions() {
    let grid = Grid::new();
    // I-piece rotation 0 cells: (dr=0, dc=-1), (0,0), (0,1), (0,2)
    // At row=0, col=4: grid coords (0,3),(0,4),(0,5),(0,6)
    // board_x=0, board_y=0, cell_size=30:
    //   col 3 → px_x=90, col 4 → 120, col 5 → 150, col 6 → 180. All px_y=0.
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    let active_quads: Vec<_> = quads.iter().filter(|q| q.4[3] == 255).collect();
    assert!(active_quads.iter().any(|q| q.0 == 90 && q.1 == 0));
    assert!(active_quads.iter().any(|q| q.0 == 120 && q.1 == 0));
    assert!(active_quads.iter().any(|q| q.0 == 150 && q.1 == 0));
    assert!(active_quads.iter().any(|q| q.0 == 180 && q.1 == 0));
}

#[test]
fn board_quads_ghost_correct_pixel_positions() {
    let grid = Grid::new();
    // I-piece at row=0, col=4, empty grid → ghost at row=19.
    // Ghost cells (19,3),(19,4),(19,5),(19,6) → px_y=570, px_x=90,120,150,180.
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    let ghost_quads: Vec<_> = quads.iter().filter(|q| q.4[3] == 80).collect();
    assert!(ghost_quads.iter().any(|q| q.0 == 90 && q.1 == 570));
    assert!(ghost_quads.iter().any(|q| q.0 == 120 && q.1 == 570));
    assert!(ghost_quads.iter().any(|q| q.0 == 150 && q.1 == 570));
    assert!(ghost_quads.iter().any(|q| q.0 == 180 && q.1 == 570));
}

#[test]
fn board_quads_active_piece_color_matches_piece_color() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let expected_color = piece_color(TetrominoType::I as u32); // [0, 255, 255, 255]
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    let active_quads: Vec<_> = quads.iter().filter(|q| q.4[3] == 255).collect();
    for q in &active_quads {
        assert_eq!(q.4, expected_color, "Active piece quad color mismatch");
    }
}

#[test]
fn board_quads_ghost_color_matches_piece_color_with_alpha_80() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let base_color = piece_color(TetrominoType::I as u32); // [0, 255, 255, 255]
    let expected_ghost_color = [base_color[0], base_color[1], base_color[2], 80];
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    let ghost_quads: Vec<_> = quads.iter().filter(|q| q.4[3] == 80).collect();
    for q in &ghost_quads {
        assert_eq!(q.4, expected_ghost_color, "Ghost quad color mismatch");
    }
}

// ---- board_quads: piece at bottom, ghost coincides with active ----

#[test]
fn board_quads_piece_at_bottom_no_ghost_quads() {
    let grid = Grid::new();
    // I-piece at row=19 (bottom). move_down would go to row 20 → out of bounds.
    // Ghost row = 19 = active row → all ghost cells coincide → no ghost quads.
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 19,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    // Only 4 active quads, 0 ghost (all coincide), 0 occupied cells.
    assert_eq!(quads.len(), 4);
    for q in &quads {
        assert_eq!(q.4[3], 255, "All quads should be active (full alpha), no ghost");
    }
}

// ---- board_quads: occupied grid cells ----

#[test]
fn board_quads_occupied_cell_included_in_result() {
    let mut grid = Grid::new();
    // Place an occupied cell at (row=5, col=3) with type_index=2 (T-piece, purple).
    grid.cells[5][3] = CellState::Occupied(2);
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    // Expected pixel position for (row=5, col=3): px_x=90, px_y=150.
    assert!(
        quads.iter().any(|q| q.0 == 90 && q.1 == 150),
        "Occupied cell at (5,3) should produce a quad at (90,150)"
    );
}

#[test]
fn board_quads_occupied_cell_color_matches_piece_color() {
    let mut grid = Grid::new();
    grid.cells[5][3] = CellState::Occupied(2);
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let expected_color = piece_color(2); // [128, 0, 128, 255]
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    let cell_quad = quads.iter().find(|q| q.0 == 90 && q.1 == 150);
    assert!(cell_quad.is_some(), "Should find quad for occupied cell");
    assert_eq!(cell_quad.unwrap().4, expected_color);
}

#[test]
fn board_quads_occupied_cell_has_full_alpha() {
    let mut grid = Grid::new();
    grid.cells[10][5] = CellState::Occupied(4);
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    // (row=10, col=5) → px (150, 300)
    let cell_quad = quads.iter().find(|q| q.0 == 150 && q.1 == 300);
    assert!(cell_quad.is_some());
    assert_eq!(cell_quad.unwrap().4[3], 255);
}

#[test]
fn board_quads_empty_cells_not_included() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::O,
        rotation: 0,
        row: 5,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    // O-piece rotation 0: (0,0),(0,1),(1,0),(1,1) → grid cells (5,4),(5,5),(6,4),(6,5)
    // Ghost: O-piece height goes down to row+1, so piece at row=5 has cell at row=6.
    // Ghost would be at the lowest valid row. Empty grid → ghost drops far below.
    // Active: 4 quads. Ghost: 4 quads (unless they overlap). No occupied cells.
    // No quad for empty non-piece cells.
    assert!(quads.len() <= 8);
    // Every quad should correspond to either an active piece cell or a ghost cell.
    // All quads have either alpha=255 (active) or alpha=80 (ghost).
    for q in &quads {
        assert!(q.4[3] == 255 || q.4[3] == 80, "Only active and ghost quads expected");
    }
}

#[test]
fn board_quads_quad_dimensions_equal_cell_size() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::T,
        rotation: 0,
        row: 5,
        col: 4,
    };
    let quads = board_quads(&grid, &piece, 0, 0, 30);
    for q in &quads {
        assert_eq!(q.2, 30, "quad width should equal cell_size");
        assert_eq!(q.3, 30, "quad height should equal cell_size");
    }
}

#[test]
fn board_quads_board_offset_applied() {
    let grid = Grid::new();
    let piece = ActivePiece {
        piece_type: TetrominoType::I,
        rotation: 0,
        row: 0,
        col: 4,
    };
    // board at (100, 200)
    let quads = board_quads(&grid, &piece, 100, 200, 30);
    // I-piece cells at (0,3),(0,4),(0,5),(0,6):
    // px_x = 100 + col*30, px_y = 200 + 0 = 200
    let active_quads: Vec<_> = quads.iter().filter(|q| q.4[3] == 255).collect();
    assert!(active_quads.iter().any(|q| q.0 == 190 && q.1 == 200)); // col=3: 100+90=190
    assert!(active_quads.iter().any(|q| q.0 == 220 && q.1 == 200)); // col=4: 100+120=220
}

// ---- next_piece_quads ----

#[test]
fn next_piece_quads_returns_4_quads() {
    for piece_type in 0..7usize {
        let quads = next_piece_quads(piece_type, 0, 0, 30);
        assert_eq!(quads.len(), 4, "piece_type {} should return 4 quads", piece_type);
    }
}

#[test]
fn next_piece_quads_uses_rotation_0_cells() {
    // For O-piece (index 1), rotation 0: (0,0),(0,1),(1,0),(1,1)
    // preview_x=0, preview_y=0, cell_size=30
    // Expected positions: (0,0),(30,0),(0,30),(30,30)
    let quads = next_piece_quads(1, 0, 0, 30);
    assert_eq!(quads.len(), 4);
    assert!(quads.iter().any(|q| q.0 == 0 && q.1 == 0));   // (dc=0)*30=0, (dr=0)*30=0
    assert!(quads.iter().any(|q| q.0 == 30 && q.1 == 0));  // (dc=1)*30=30, (dr=0)*30=0
    assert!(quads.iter().any(|q| q.0 == 0 && q.1 == 30));  // (dc=0)*30=0, (dr=1)*30=30
    assert!(quads.iter().any(|q| q.0 == 30 && q.1 == 30)); // (dc=1)*30=30, (dr=1)*30=30
}

#[test]
fn next_piece_quads_i_piece_rotation_0_positions() {
    // I-piece (index 0), rotation 0: (dr=0,dc=-1),(0,0),(0,1),(0,2)
    // x = preview_x + dc*cell_size, y = preview_y + dr*cell_size
    // preview_x=0, preview_y=0, cell_size=30:
    // (-1)*30=-30, 0, 30, 60 for x. All y=0.
    let quads = next_piece_quads(0, 0, 0, 30);
    assert!(quads.iter().any(|q| q.0 == -30 && q.1 == 0));
    assert!(quads.iter().any(|q| q.0 == 0 && q.1 == 0));
    assert!(quads.iter().any(|q| q.0 == 30 && q.1 == 0));
    assert!(quads.iter().any(|q| q.0 == 60 && q.1 == 0));
}

#[test]
fn next_piece_quads_respects_preview_offset() {
    // T-piece (index 2), rotation 0: (-1,0),(0,-1),(0,0),(0,1)
    // preview_x=100, preview_y=200, cell_size=30
    // x = 100 + dc*30, y = 200 + dr*30
    let cells_r0 = PIECE_CELLS[2][0]; // T-piece rotation 0
    let quads = next_piece_quads(2, 100, 200, 30);
    for (dr, dc) in &cells_r0 {
        let expected_x = 100 + dc * 30;
        let expected_y = 200 + dr * 30;
        assert!(
            quads.iter().any(|q| q.0 == expected_x && q.1 == expected_y),
            "Missing quad at ({}, {}) for T-piece rotation 0 cell ({}, {})",
            expected_x, expected_y, dr, dc
        );
    }
}

#[test]
fn next_piece_quads_correct_color() {
    for piece_type in 0..7usize {
        let quads = next_piece_quads(piece_type, 0, 0, 30);
        let expected = piece_color(piece_type as u32);
        for q in &quads {
            assert_eq!(
                q.4, expected,
                "piece_type {} quad color mismatch",
                piece_type
            );
        }
    }
}

#[test]
fn next_piece_quads_all_same_size() {
    for piece_type in 0..7usize {
        let quads = next_piece_quads(piece_type, 0, 0, 30);
        for q in &quads {
            assert_eq!(q.2, 30, "piece_type {} quad width should be 30", piece_type);
            assert_eq!(q.3, 30, "piece_type {} quad height should be 30", piece_type);
        }
    }
}

#[test]
fn next_piece_quads_custom_cell_size() {
    // O-piece with cell_size=16: positions at 0, 16, 0, 16 / 0, 0, 16, 16
    let quads = next_piece_quads(1, 0, 0, 16);
    assert_eq!(quads.len(), 4);
    for q in &quads {
        assert_eq!(q.2, 16);
        assert_eq!(q.3, 16);
    }
    // Check one expected position: (dc=1)*16=16, (dr=0)*16=0
    assert!(quads.iter().any(|q| q.0 == 16 && q.1 == 0));
}

#[test]
fn next_piece_quads_full_alpha() {
    // next_piece_quads should return full alpha (255) for all quads.
    for piece_type in 0..7usize {
        let quads = next_piece_quads(piece_type, 0, 0, 30);
        for q in &quads {
            assert_eq!(q.4[3], 255, "piece_type {} next piece quad should have full alpha", piece_type);
        }
    }
}
