use freecut::{
    domain::{
        CutPiece, CutSettings, LayoutKind, LinearKerf, PatternDirection, PieceId, Project,
        StockPiece, Unit,
    },
    optimizer::{BaselineOptimizer, OptimizeError, OptimizerConfig, OptimizerEffort},
    render::{Cut, Rect, SliceNode, Solution},
};

#[test]
fn balanced_optimizer_places_gui_case_after_adding_one_more_small_cut() {
    let project = Project {
        name: "repro".to_string(),
        stock_pieces: vec![StockPiece {
            id: PieceId(1),
            width: 2440,
            length: 1220,
            quantity: Some(1),
            pattern: PatternDirection::None,
        }],
        cut_pieces: vec![
            CutPiece {
                id: PieceId(2),
                label: "cut-2".to_string(),
                width: 800,
                length: 100,
                quantity: 15,
                pattern: PatternDirection::None,
                can_rotate: true,
            },
            CutPiece {
                id: PieceId(3),
                label: "cut-3".to_string(),
                width: 100,
                length: 78,
                quantity: 44,
                pattern: PatternDirection::None,
                can_rotate: true,
            },
            CutPiece {
                id: PieceId(4),
                label: "cut-4".to_string(),
                width: 200,
                length: 60,
                quantity: 10,
                pattern: PatternDirection::None,
                can_rotate: true,
            },
            CutPiece {
                id: PieceId(5),
                label: "cut-5".to_string(),
                width: 150,
                length: 150,
                quantity: 35,
                pattern: PatternDirection::None,
                can_rotate: true,
            },
        ],
        settings: CutSettings {
            unit: Unit::Millimeter,
            kerf_width: 0,
            linear_kerf: None,
            layout: LayoutKind::Guillotine,
        },
    };

    let solution = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Balanced))
        .expect("should fit");

    assert_eq!(solution.sheets.len(), 1);
    assert_eq!(solution.sheets[0].placed_pieces.len(), 104);
}

#[test]
fn thorough_guillotine_places_q71_side_strip_case() {
    let project = Project {
        name: "side-strip-repro".to_string(),
        stock_pieces: vec![StockPiece {
            id: PieceId(2),
            width: 2440,
            length: 1220,
            quantity: Some(1),
            pattern: PatternDirection::None,
        }],
        cut_pieces: vec![
            CutPiece {
                id: PieceId(1),
                label: "cut-1".to_string(),
                width: 100,
                length: 100,
                quantity: 85,
                pattern: PatternDirection::None,
                can_rotate: true,
            },
            CutPiece {
                id: PieceId(3),
                label: "cut-3".to_string(),
                width: 234,
                length: 344,
                quantity: 21,
                pattern: PatternDirection::None,
                can_rotate: true,
            },
            CutPiece {
                id: PieceId(4),
                label: "cut-4".to_string(),
                width: 40,
                length: 120,
                quantity: 71,
                pattern: PatternDirection::None,
                can_rotate: true,
            },
        ],
        settings: CutSettings {
            unit: Unit::Millimeter,
            kerf_width: 1,
            linear_kerf: None,
            layout: LayoutKind::Guillotine,
        },
    };

    let solution = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Thorough))
        .expect("q71 side-strip case should fit on one sheet");

    assert_eq!(solution.sheets.len(), 1);
    assert_eq!(solution.sheets[0].placed_pieces.len(), 177);
    assert_eq!(
        solution.sheets[0]
            .placed_pieces
            .iter()
            .filter(|piece| piece.cut_id == PieceId(1))
            .count(),
        85
    );
    assert_eq!(
        solution.sheets[0]
            .placed_pieces
            .iter()
            .filter(|piece| piece.cut_id == PieceId(3))
            .count(),
        21
    );
    assert_eq!(
        solution.sheets[0]
            .placed_pieces
            .iter()
            .filter(|piece| piece.cut_id == PieceId(4))
            .count(),
        71
    );
}

#[test]
fn nested_places_small_non_guillotine_advantage_case() {
    let nested = nested_project(
        10,
        10,
        vec![
            cut(10, 2, 2, 1, false),
            cut(20, 2, 3, 1, false),
            cut(30, 2, 6, 1, false),
            cut(40, 7, 8, 1, false),
        ],
    );
    let mut guillotine = nested.clone();
    guillotine.settings.layout = LayoutKind::Guillotine;

    let nested_solution = BaselineOptimizer
        .optimize_with_config(&nested, OptimizerConfig::new(OptimizerEffort::Thorough))
        .expect("nested should place this small non-guillotine layout");
    let guillotine_error = BaselineOptimizer
        .optimize_with_config(&guillotine, OptimizerConfig::new(OptimizerEffort::Thorough))
        .expect_err("guillotine should not place this non-sliceable layout with one sheet");

    assert_eq!(guillotine_error, OptimizeError::NoSolution);
    assert_eq!(nested_solution.sheets.len(), 1);
    assert_eq!(nested_solution.sheets[0].placed_pieces.len(), 4);
    assert_solution_within_bounds_and_non_overlapping(&nested_solution);
}

#[test]
fn nested_rotates_narrow_piece_into_remaining_strip() {
    let project = nested_project(
        100,
        60,
        vec![cut(10, 60, 60, 1, false), cut(20, 50, 40, 1, true)],
    );

    let solution = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Thorough))
        .expect("rotated narrow piece should fit in the side strip");

    assert_eq!(solution.sheets.len(), 1);
    assert_eq!(solution.sheets[0].placed_pieces.len(), 2);
    assert!(solution.sheets[0]
        .placed_pieces
        .iter()
        .any(|piece| piece.cut_id == PieceId(20) && piece.rotated));
    assert_solution_within_bounds_and_non_overlapping(&solution);
}

#[test]
fn nested_uses_pattern_wildcard_like_guillotine() {
    let project = Project {
        name: "nested-pattern-wildcard".to_string(),
        stock_pieces: vec![StockPiece {
            id: PieceId(1),
            width: 100,
            length: 100,
            quantity: Some(1),
            pattern: PatternDirection::ParallelToWidth,
        }],
        cut_pieces: vec![cut(10, 50, 50, 1, false)],
        settings: CutSettings {
            unit: Unit::Millimeter,
            kerf_width: 0,
            linear_kerf: None,
            layout: LayoutKind::Nested,
        },
    };

    let solution = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("patternless cut should fit patterned stock");

    assert_eq!(solution.sheets.len(), 1);
    assert_eq!(solution.sheets[0].placed_pieces.len(), 1);
    assert_eq!(
        solution.sheets[0].placed_pieces[0].pattern,
        PatternDirection::None
    );
}

#[test]
fn nested_respects_finite_stock_quantity() {
    let project = nested_project(100, 100, vec![cut(10, 60, 100, 2, false)]);

    let error = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Thorough))
        .expect_err("one finite stock sheet cannot hold both cuts");

    assert_eq!(error, OptimizeError::NoSolution);
}

#[test]
fn nested_rejects_when_total_cut_area_exceeds_stock_area() {
    let project = nested_project(100, 100, vec![cut(10, 80, 80, 2, false)]);

    let error = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect_err("total cut area exceeds available stock area");

    assert_eq!(error, OptimizeError::NoSolution);
}

#[test]
fn nested_rejects_individually_unpassable_cut() {
    let project = nested_project(100, 100, vec![cut(10, 101, 50, 1, false)]);

    let error = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect_err("cut is wider than any stock piece");

    assert_eq!(error, OptimizeError::NoSolution);
}

#[test]
fn nested_repeats_deterministically_for_all_effort_levels() {
    let project = nested_project(
        180,
        120,
        vec![
            cut(10, 70, 40, 1, true),
            cut(20, 60, 50, 1, true),
            cut(30, 30, 80, 1, true),
            cut(40, 25, 25, 3, true),
        ],
    );

    for effort in [
        OptimizerEffort::Fast,
        OptimizerEffort::Balanced,
        OptimizerEffort::Thorough,
    ] {
        let first = BaselineOptimizer
            .optimize_with_config(&project, OptimizerConfig::new(effort))
            .expect("first nested run should produce a solution");
        let second = BaselineOptimizer
            .optimize_with_config(&project, OptimizerConfig::new(effort))
            .expect("second nested run should produce a solution");

        assert_eq!(first, second);
    }
}

#[test]
fn guillotine_balanced_places_rotation_disabled_small_cut_cases() {
    for disabled_cut_id in [4, 5] {
        let project = rotation_disabled_regression_project(LayoutKind::Guillotine, disabled_cut_id);

        let fast_error = BaselineOptimizer
            .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
            .expect_err("fast may remain a narrow greedy guillotine pass");
        let balanced_solution = BaselineOptimizer
            .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Balanced))
            .expect("balanced should include enough guillotine variants for this practical case");

        assert_eq!(fast_error, OptimizeError::NoSolution);
        assert_eq!(balanced_solution.sheets.len(), 1);
        assert_eq!(balanced_solution.sheets[0].placed_pieces.len(), 47);
        assert_solution_within_bounds_and_non_overlapping(&balanced_solution);
    }
}

#[test]
fn nested_balanced_and_thorough_place_wide_panel_rotation_disabled_case() {
    let project = rotation_disabled_regression_project(LayoutKind::Nested, 2);

    for effort in [OptimizerEffort::Balanced, OptimizerEffort::Thorough] {
        let solution = BaselineOptimizer
            .optimize_with_config(&project, OptimizerConfig::new(effort))
            .expect("nested should handle the wide-panel case without requiring rotation");

        assert_eq!(solution.sheets.len(), 1);
        assert_eq!(solution.sheets[0].placed_pieces.len(), 47);
        assert_solution_within_bounds_and_non_overlapping(&solution);
    }
}

fn nested_project(stock_width: u32, stock_length: u32, cut_pieces: Vec<CutPiece>) -> Project {
    Project {
        name: "nested-regression".to_string(),
        stock_pieces: vec![StockPiece {
            id: PieceId(1),
            width: stock_width,
            length: stock_length,
            quantity: Some(1),
            pattern: PatternDirection::None,
        }],
        cut_pieces,
        settings: CutSettings {
            unit: Unit::Millimeter,
            kerf_width: 0,
            linear_kerf: None,
            layout: LayoutKind::Nested,
        },
    }
}

fn rotation_disabled_regression_project(layout: LayoutKind, disabled_cut_id: u64) -> Project {
    Project {
        name: "rotation-disabled-regression".to_string(),
        stock_pieces: vec![StockPiece {
            id: PieceId(1),
            width: 2440,
            length: 1220,
            quantity: Some(1),
            pattern: PatternDirection::None,
        }],
        cut_pieces: vec![
            cut(2, 500, 620, 4, disabled_cut_id != 2),
            cut(3, 1223, 220, 3, disabled_cut_id != 3),
            cut(4, 110, 100, 30, disabled_cut_id != 4),
            cut(5, 100, 200, 10, disabled_cut_id != 5),
        ],
        settings: CutSettings {
            unit: Unit::Millimeter,
            kerf_width: 2,
            linear_kerf: None,
            layout,
        },
    }
}

fn cut(id: u64, width: u32, length: u32, quantity: u32, can_rotate: bool) -> CutPiece {
    CutPiece {
        id: PieceId(id),
        label: format!("cut-{id}"),
        width,
        length,
        quantity,
        pattern: PatternDirection::None,
        can_rotate,
    }
}

fn assert_solution_within_bounds_and_non_overlapping(solution: &freecut::render::Solution) {
    for sheet in &solution.sheets {
        for (index, placed) in sheet.placed_pieces.iter().enumerate() {
            assert!(
                placed.rect.x + placed.rect.width <= sheet.width,
                "placed piece {index} exceeds sheet width"
            );
            assert!(
                placed.rect.y + placed.rect.length <= sheet.length,
                "placed piece {index} exceeds sheet length"
            );

            for other in sheet.placed_pieces.iter().skip(index + 1) {
                assert!(
                    !rects_overlap(placed.rect, other.rect),
                    "placed pieces overlap: {:?} and {:?}",
                    placed.rect,
                    other.rect
                );
            }
        }
    }
}

fn rects_overlap(left: Rect, right: Rect) -> bool {
    left.x < right.x + right.width
        && left.x + left.width > right.x
        && left.y < right.y + right.length
        && left.y + left.length > right.y
}

fn linear_kerf_base_project() -> Project {
    Project {
        name: "linear-kerf".to_string(),
        stock_pieces: vec![StockPiece {
            id: PieceId(1),
            width: 1000,
            length: 1000,
            quantity: Some(1),
            pattern: PatternDirection::None,
        }],
        cut_pieces: vec![CutPiece {
            id: PieceId(2),
            label: "cell".to_string(),
            width: 100,
            length: 100,
            quantity: 25,
            pattern: PatternDirection::None,
            can_rotate: true,
        }],
        settings: CutSettings {
            unit: Unit::Millimeter,
            kerf_width: 0,
            linear_kerf: None,
            layout: LayoutKind::Guillotine,
        },
    }
}

fn collect_cuts(node: &SliceNode, out: &mut Vec<Cut>) {
    if let SliceNode::Cut { cut, first, second } = node {
        out.push(*cut);
        collect_cuts(first, out);
        collect_cuts(second, out);
    }
}

fn solution_cuts(solution: &Solution) -> Vec<Cut> {
    let mut cuts = Vec::new();
    for sheet in &solution.sheets {
        if let Some(tree) = &sheet.cutting_guide {
            collect_cuts(tree, &mut cuts);
        }
    }
    cuts
}

#[test]
fn linear_kerf_zero_reference_treated_as_none() {
    let mut project = linear_kerf_base_project();
    project.settings.kerf_width = 2;

    let baseline = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("baseline should fit");

    project.settings.linear_kerf = Some(LinearKerf {
        extra: 5,
        reference: 0,
    });
    let zero_reference = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("zero-reference should fit");

    assert_eq!(baseline, zero_reference);
}

#[test]
fn linear_kerf_ignored_for_nested() {
    let mut project = linear_kerf_base_project();
    project.settings.layout = LayoutKind::Nested;
    project.settings.kerf_width = 2;

    let without = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("nested without linear should fit");

    project.settings.linear_kerf = Some(LinearKerf {
        extra: 50,
        reference: 1,
    });
    let with_linear = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("nested with linear should fit");

    assert_eq!(without, with_linear);
}

#[test]
fn linear_kerf_increases_total_cut_kerf_in_slicing_tree() {
    let mut project = linear_kerf_base_project();

    let baseline = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("baseline should fit");
    let baseline_kerf_sum: u64 = solution_cuts(&baseline)
        .iter()
        .map(|c| u64::from(c.kerf_width()))
        .sum();

    project.settings.linear_kerf = Some(LinearKerf {
        extra: 5,
        reference: 100,
    });
    let with_linear = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("linear-kerf project should fit");
    let linear_kerf_sum: u64 = solution_cuts(&with_linear)
        .iter()
        .map(|c| u64::from(c.kerf_width()))
        .sum();

    assert!(
        linear_kerf_sum > baseline_kerf_sum,
        "linear kerf should widen cuts: baseline={baseline_kerf_sum} linear={linear_kerf_sum}"
    );
}

#[test]
fn linear_kerf_widens_cuts_in_proportion_to_length() {
    let mut project = linear_kerf_base_project();
    project.settings.linear_kerf = Some(LinearKerf {
        extra: 5,
        reference: 100,
    });

    let solution = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Fast))
        .expect("variable-kerf project should fit");

    let cuts = solution_cuts(&solution);
    assert!(!cuts.is_empty(), "expected cutting guide to contain cuts");

    for cut in &cuts {
        let length = match cut.orientation() {
            freecut::render::CutOrientation::Horizontal => cut.work_rect().width,
            freecut::render::CutOrientation::Vertical => cut.work_rect().length,
        };
        let expected = (5u64 * u64::from(length) / 100) as u32;
        assert_eq!(
            cut.kerf_width(),
            expected,
            "cut on work_rect {:?} oriented {:?} should have kerf {expected} for length {length}, got {}",
            cut.work_rect(),
            cut.orientation(),
            cut.kerf_width()
        );
    }

    assert!(
        cuts.iter().any(|c| c.kerf_width() > 0),
        "expected at least one non-zero kerf cut"
    );
}

#[test]
fn linear_kerf_solution_passes_internal_geometry_assertions() {
    let mut project = linear_kerf_base_project();
    project.settings.kerf_width = 1;
    project.settings.linear_kerf = Some(LinearKerf {
        extra: 3,
        reference: 200,
    });

    let solution = BaselineOptimizer
        .optimize_with_config(&project, OptimizerConfig::new(OptimizerEffort::Balanced))
        .expect("variable-kerf project should fit");

    assert!(!solution.sheets.is_empty());
    assert!(solution.sheets[0].cutting_guide.is_some());
}
