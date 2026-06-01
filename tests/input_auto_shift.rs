// @spec-tags: input,controls,timing,movement
// @invariants: DAS/ARR horizontal auto-shift timing state machine (ShiftDir, ShiftMove, AutoShift)
// @build: 102

use rhythm_grid::input::{AutoShift, ShiftDir, ShiftMove};

// --- Construction ---

#[test]
fn auto_shift_new_starts_with_no_active_dir() {
    let a = AutoShift::new(100.0, 20.0);
    assert_eq!(a.active_dir(), None);
}

#[test]
fn auto_shift_default_starts_with_no_active_dir() {
    let a = AutoShift::default();
    assert_eq!(a.active_dir(), None);
}

// --- active_dir ---

#[test]
fn active_dir_none_before_any_press() {
    let mut a = AutoShift::new(150.0, 33.0);
    assert_eq!(a.active_dir(), None);
}

#[test]
fn active_dir_set_to_left_after_press_left() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    assert_eq!(a.active_dir(), Some(ShiftDir::Left));
}

#[test]
fn active_dir_set_to_right_after_press_right() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Right);
    assert_eq!(a.active_dir(), Some(ShiftDir::Right));
}

#[test]
fn active_dir_cleared_on_release_with_no_opposite_held() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    a.release(ShiftDir::Left);
    assert_eq!(a.active_dir(), None);
}

// --- Initial tap ---

#[test]
fn initial_tap_emits_steps_1_for_left() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let result = a.update(0.0);
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::Steps(1))));
}

#[test]
fn initial_tap_emits_steps_1_for_right() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Right);
    let result = a.update(0.0);
    assert_eq!(result, Some((ShiftDir::Right, ShiftMove::Steps(1))));
}

#[test]
fn no_press_yields_none_from_update() {
    let mut a = AutoShift::new(150.0, 33.0);
    assert_eq!(a.update(100.0), None);
}

#[test]
fn press_itself_does_not_emit_movement() {
    // press() alone produces no output; output only comes from update()
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    // No update call yet — active_dir is set but no movement was emitted during press
    assert_eq!(a.active_dir(), Some(ShiftDir::Left));
}

#[test]
fn initial_tap_emitted_only_on_first_update() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let first = a.update(0.0);
    assert_eq!(first, Some((ShiftDir::Left, ShiftMove::Steps(1))));
    // Second update within DAS window — initial tap already consumed
    let second = a.update(1.0);
    assert_eq!(second, None);
}

// --- DAS delay ---

#[test]
fn no_arr_repeat_before_das_threshold() {
    // das=150, arr=33: after initial tap, 149ms elapsed is still within DAS — no output
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    let result = a.update(149.0);
    assert_eq!(result, None);
}

#[test]
fn arr_step_fires_after_das_threshold_crossed() {
    // das=150, arr=33: initial tap consumed; advance 149ms (within DAS) then 34ms (past DAS+ARR)
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    let _within_das = a.update(149.0); // 149ms total, still inside DAS
    // Now 149+34=183ms >= 150+33 — one full ARR interval past DAS
    let result = a.update(34.0);
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::Steps(1))));
}

#[test]
fn arr_large_dt_yields_multiple_steps() {
    // das=50, arr=10: after initial tap, a single 100ms frame spans 5 ARR intervals (50..99ms past DAS)
    let mut a = AutoShift::new(50.0, 10.0);
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    // 100ms elapsed: floor((100 - 50) / 10) = 5 steps
    let result = a.update(100.0);
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::Steps(5))));
}

#[test]
fn arr_cumulative_time_not_lost_across_frames() {
    // das=10, arr=10: partial frames don't lose fractional ARR time
    let mut a = AutoShift::new(10.0, 10.0);
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    // 10ms: at DAS boundary exactly, floor(0/10) = 0 ARR steps -> None
    assert_eq!(a.update(10.0), None);
    // 15ms total: 5ms past DAS, floor(5/10) = 0 -> None
    assert_eq!(a.update(5.0), None);
    // 20ms total: 10ms past DAS, floor(10/10) = 1 -> Steps(1)
    assert_eq!(a.update(5.0), Some((ShiftDir::Left, ShiftMove::Steps(1))));
}

#[test]
fn arr_cumulative_only_new_steps_emitted_per_frame() {
    // Ensure previously-emitted ARR steps aren't re-emitted
    let mut a = AutoShift::new(10.0, 10.0);
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    // First ARR step at 20ms total
    let _first_arr = a.update(20.0); // Steps(1): floor(10/10) = 1
    // Next ARR step at 30ms total: floor(20/10)=2 total, minus 1 already emitted = 1 new
    let second_arr = a.update(10.0);
    assert_eq!(second_arr, Some((ShiftDir::Left, ShiftMove::Steps(1))));
}

// --- Instant charge (arr_ms == 0) ---

#[test]
fn instant_charge_emits_charge_to_wall_after_das() {
    // das=50, arr=0: initial tap first, then ChargeToWall when held time >= das
    let mut a = AutoShift::new(50.0, 0.0);
    a.press(ShiftDir::Left);
    let tap = a.update(0.0);
    assert_eq!(tap, Some((ShiftDir::Left, ShiftMove::Steps(1))));
    let result = a.update(60.0); // 60ms >= das(50ms)
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::ChargeToWall)));
}

#[test]
fn instant_charge_direction_matches_active_dir() {
    let mut a = AutoShift::new(50.0, 0.0);
    a.press(ShiftDir::Right);
    let _tap = a.update(0.0);
    let result = a.update(60.0);
    assert_eq!(result, Some((ShiftDir::Right, ShiftMove::ChargeToWall)));
}

#[test]
fn instant_charge_latches_subsequent_updates_return_none() {
    let mut a = AutoShift::new(50.0, 0.0);
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    let charge = a.update(60.0);
    assert_eq!(charge, Some((ShiftDir::Left, ShiftMove::ChargeToWall)));
    // Latch: further updates emit None while key held
    assert_eq!(a.update(100.0), None);
    assert_eq!(a.update(100.0), None);
}

#[test]
fn instant_charge_das_zero_arr_zero_first_update_is_charge_to_wall() {
    // das=0, arr=0: initial tap is subsumed; first update -> ChargeToWall
    let mut a = AutoShift::new(0.0, 0.0);
    a.press(ShiftDir::Left);
    let result = a.update(0.0);
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::ChargeToWall)));
}

#[test]
fn instant_charge_das_zero_arr_zero_latches_after_first_update() {
    let mut a = AutoShift::new(0.0, 0.0);
    a.press(ShiftDir::Left);
    let _charge = a.update(0.0);
    assert_eq!(a.update(0.0), None);
    assert_eq!(a.update(10.0), None);
}

// --- Combined initial tap + ARR in one frame (das=0, arr>0) ---

#[test]
fn initial_tap_and_arr_combine_in_one_frame_when_das_is_zero() {
    // das=0, arr=10: DAS threshold is immediately met on press.
    // First update(50ms): initial tap (1) + floor(50/10)=5 ARR steps = Steps(6)
    let mut a = AutoShift::new(0.0, 10.0);
    a.press(ShiftDir::Left);
    let result = a.update(50.0);
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::Steps(6))));
}

// --- Last-key-wins ---

#[test]
fn last_key_wins_second_press_overrides_first() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _left_tap = a.update(0.0);
    let _ = a.update(100.0); // advance into DAS for Left
    // Press Right: Right becomes active, DAS resets
    a.press(ShiftDir::Right);
    assert_eq!(a.active_dir(), Some(ShiftDir::Right));
    let result = a.update(0.0); // initial tap for Right
    assert_eq!(result, Some((ShiftDir::Right, ShiftMove::Steps(1))));
}

#[test]
fn last_key_wins_das_resets_on_direction_switch() {
    // After switching direction, DAS should recharge from zero — no ARR before 150ms
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _left_tap = a.update(0.0);
    let _ = a.update(200.0); // Left: well past DAS
    a.press(ShiftDir::Right); // switch to Right
    let _right_tap = a.update(0.0); // Right initial tap
    // 149ms later — still within Right's fresh DAS
    let within_das = a.update(149.0);
    assert_eq!(within_das, None);
}

// --- Release behavior ---

#[test]
fn release_active_dir_with_no_opposite_yields_none_from_update() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    a.release(ShiftDir::Left);
    assert_eq!(a.active_dir(), None);
    assert_eq!(a.update(0.0), None);
}

#[test]
fn release_active_reverts_to_still_held_opposite() {
    // press Left, press Right (Right active), release Right -> reverts to Left
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _ = a.update(0.0); // consume Left initial tap
    a.press(ShiftDir::Right);
    let _ = a.update(0.0); // consume Right initial tap
    a.release(ShiftDir::Right); // Left still held; revert
    assert_eq!(a.active_dir(), Some(ShiftDir::Left));
    // Revert counts as a fresh press: initial tap for Left
    let result = a.update(0.0);
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::Steps(1))));
}

#[test]
fn release_non_active_dir_does_not_change_active() {
    // press Left then Right (Right active), release Left (non-active) — Right remains active
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    a.press(ShiftDir::Right); // Right is active, Left still held
    a.release(ShiftDir::Left); // release non-active Left
    assert_eq!(a.active_dir(), Some(ShiftDir::Right));
}

#[test]
fn release_non_active_then_release_active_clears() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    a.press(ShiftDir::Right);
    a.release(ShiftDir::Left); // remove Left from held
    a.release(ShiftDir::Right); // Right released, no opposite held -> None
    assert_eq!(a.active_dir(), None);
}

#[test]
fn release_revert_recharges_das_from_zero() {
    // After reverting to Left, its DAS recharges from scratch
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _ = a.update(0.0); // Left tap
    let _ = a.update(200.0); // Left: well past DAS + ARR
    a.press(ShiftDir::Right);
    let _ = a.update(0.0); // Right tap
    a.release(ShiftDir::Right); // revert to Left, fresh DAS
    assert_eq!(a.active_dir(), Some(ShiftDir::Left));
    let tap = a.update(0.0); // fresh initial tap for Left
    assert_eq!(tap, Some((ShiftDir::Left, ShiftMove::Steps(1))));
    // 100ms later — within Left's fresh DAS (150ms) -> None
    assert_eq!(a.update(100.0), None);
}

// --- Reset ---

#[test]
fn reset_clears_active_dir() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    a.reset();
    assert_eq!(a.active_dir(), None);
}

#[test]
fn reset_stops_update_output() {
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _ = a.update(0.0);
    a.reset();
    assert_eq!(a.update(0.0), None);
    assert_eq!(a.update(1000.0), None);
}

#[test]
fn reset_clears_instant_charge_latch() {
    // After a ChargeToWall latch, reset should allow a new press+charge
    let mut a = AutoShift::new(0.0, 0.0);
    a.press(ShiftDir::Left);
    let charge = a.update(0.0);
    assert_eq!(charge, Some((ShiftDir::Left, ShiftMove::ChargeToWall)));
    assert_eq!(a.update(0.0), None); // latched
    a.reset();
    a.press(ShiftDir::Right);
    let new_charge = a.update(0.0);
    assert_eq!(new_charge, Some((ShiftDir::Right, ShiftMove::ChargeToWall)));
}

#[test]
fn reset_clears_das_timer() {
    // After reset, a new press starts DAS from zero
    let mut a = AutoShift::new(150.0, 33.0);
    a.press(ShiftDir::Left);
    let _ = a.update(0.0);
    let _ = a.update(200.0); // past DAS
    a.reset();
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0); // initial tap
    // 100ms later: within fresh DAS -> None (not an ARR repeat from old timer)
    assert_eq!(a.update(100.0), None);
}

// --- Default constants ---

#[test]
fn default_das_is_150ms_behaviorally() {
    // With default AutoShift, no ARR step before 150ms after initial tap
    let mut a = AutoShift::default();
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    assert_eq!(a.update(149.0), None); // 149ms total — still inside default DAS
}

#[test]
fn default_arr_is_33ms_behaviorally() {
    // At default DAS (150ms) + 32ms -> still no ARR; at +1ms more crosses first ARR interval
    let mut a = AutoShift::default();
    a.press(ShiftDir::Left);
    let _tap = a.update(0.0);
    let _ = a.update(150.0); // at DAS boundary: floor(0/33) = 0 ARR steps -> None
    assert_eq!(a.update(32.0), None); // 32ms past DAS: floor(32/33) = 0 -> None
    let result = a.update(1.0); // 33ms past DAS: floor(33/33) = 1 -> Steps(1)
    assert_eq!(result, Some((ShiftDir::Left, ShiftMove::Steps(1))));
}

// --- Type derives ---

#[test]
fn shift_dir_clone_copy_partialeq_eq_debug() {
    let d = ShiftDir::Left;
    let d2 = d; // Copy
    let d3 = d.clone(); // Clone
    assert_eq!(d, d2); // PartialEq + Eq
    assert_eq!(d, d3);
    assert_ne!(ShiftDir::Left, ShiftDir::Right);
    let _ = format!("{:?}", d); // Debug
}

#[test]
fn shift_move_partialeq_debug() {
    assert_eq!(ShiftMove::Steps(1), ShiftMove::Steps(1));
    assert_ne!(ShiftMove::Steps(1), ShiftMove::Steps(2));
    assert_ne!(ShiftMove::Steps(1), ShiftMove::ChargeToWall);
    assert_eq!(ShiftMove::ChargeToWall, ShiftMove::ChargeToWall);
    let _ = format!("{:?}", ShiftMove::Steps(3)); // Debug
    let _ = format!("{:?}", ShiftMove::ChargeToWall);
}

#[test]
fn shift_move_steps_carries_u32_value() {
    assert_eq!(ShiftMove::Steps(0), ShiftMove::Steps(0));
    assert_eq!(ShiftMove::Steps(u32::MAX), ShiftMove::Steps(u32::MAX));
}
