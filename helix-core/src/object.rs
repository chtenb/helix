use crate::{movement::Direction, syntax::TreeCursor, Range, RopeSlice, Selection, Syntax};

pub fn expand_selection(syntax: &Syntax, text: RopeSlice, range: Range) -> Range {
    select_node_impl(
        syntax,
        text,
        range,
        |cursor, byte_range, _| {
            while cursor.node().byte_range() == byte_range {
                if !cursor.goto_parent(false) {
                    break;
                }
            }
            return 0;
        },
        None,
    )
}

pub fn shrink_selection(syntax: &Syntax, text: RopeSlice, range: Range) -> Range {
    select_node_impl(
        syntax,
        text,
        range,
        |cursor, _, _| {
            cursor.goto_first_child(false);
            return 0;
        },
        None,
    )
}

pub fn select_all_siblings(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    selection.transform_iter(|range| {
        let mut cursor = syntax.walk();
        let (from, to) = range.into_byte_range(text);
        cursor.reset_to_byte_range(from, to);

        if !cursor.goto_parent_with(|parent| parent.child_count() > 1) {
            return vec![range].into_iter();
        }

        select_children(&mut cursor, text, range).into_iter()
    })
}

pub fn select_all_children(syntax: &Syntax, text: RopeSlice, selection: Selection) -> Selection {
    selection.transform_iter(|range| {
        let mut cursor = syntax.walk();
        let (from, to) = range.into_byte_range(text);
        cursor.reset_to_byte_range(from, to);
        select_children(&mut cursor, text, range).into_iter()
    })
}

fn select_children<'n>(
    cursor: &'n mut TreeCursor<'n>,
    text: RopeSlice,
    range: Range,
) -> Vec<Range> {
    let children = cursor
        .named_children()
        .map(|child| Range::from_node(child, text, range.direction()))
        .collect::<Vec<_>>();

    if !children.is_empty() {
        children
    } else {
        vec![range]
    }
}

pub fn select_next_sibling(syntax: &Syntax, text: RopeSlice, range: Range, named: bool) -> Range {
    select_node_impl(
        syntax,
        text,
        range,
        |cursor, byte_range, depth| {
            let mut d = depth;
            while !cursor.goto_next_sibling(named) {
                log::error!("Moving up");
                if !cursor.goto_parent(named) {
                    cursor.reset_to_byte_range(byte_range.start, byte_range.end);
                    return depth;
                }
                d += 1;
            }
            while d > 0 && cursor.goto_first_child(named) {
                log::error!("Moving down");
                d -= 1;
            }
            return d;
        },
        Some(Direction::Forward),
    )
}

pub fn select_prev_sibling(syntax: &Syntax, text: RopeSlice, range: Range, named: bool) -> Range {
    select_node_impl(
        syntax,
        text,
        range,
        |cursor, byte_range, depth| {
            let mut d = depth;
            while !cursor.goto_prev_sibling(named) {
                log::error!("Moving up");
                if !cursor.goto_parent(named) {
                    cursor.reset_to_byte_range(byte_range.start, byte_range.end);
                    return depth;
                }
                d += 1;
            }
            while d > 0 && cursor.goto_last_child(named) {
                log::error!("Moving down");
                d -= 1;
            }
            return d;
        },
        Some(Direction::Backward),
    )
}

fn select_node_impl<F>(
    syntax: &Syntax,
    text: RopeSlice,
    range: Range,
    motion: F,
    direction: Option<Direction>,
) -> Range
where
    F: Fn(&mut TreeCursor, std::ops::Range<usize>, u32) -> u32,
{
    let cursor = &mut syntax.walk();

    let from = text.char_to_byte(range.from());
    let to = text.char_to_byte(range.to());

    let byte_range = from..to;
    cursor.reset_to_byte_range(from, to);

    let old_depth = motion(cursor, byte_range, range.old_tree_depth.unwrap_or(0));
    log::error!("depth: {}", old_depth);

    let node = cursor.node();
    let from = text.byte_to_char(node.start_byte());
    let to = text.byte_to_char(node.end_byte());

    let mut result =
        Range::new(from, to).with_direction(direction.unwrap_or_else(|| range.direction()));
    result.old_tree_depth = Some(old_depth);
    return result;
}
