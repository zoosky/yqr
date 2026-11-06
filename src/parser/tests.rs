use super::*;
use crate::Value;
use crate::ast::{BinOp, Mutation, Rhs};

#[test]
fn parses_identity() {
    assert_eq!(parse(".").unwrap(), Ast::Identity);
    assert_eq!(parse("").unwrap(), Ast::Identity);
}

#[test]
fn parses_single_field() {
    assert_eq!(parse(".foo").unwrap(), Ast::Field("foo".into()));
}

#[test]
fn parses_chained_fields() {
    assert_eq!(
        parse(".a.b").unwrap(),
        Ast::pipe(Ast::Field("a".into()), Ast::Field("b".into()))
    );
}

#[test]
fn parses_index_and_iterate() {
    assert_eq!(parse(".[0]").unwrap(), Ast::Index(0));
    assert_eq!(parse(".[-1]").unwrap(), Ast::Index(-1));
    assert_eq!(parse(".[]").unwrap(), Ast::Iterate);
}

#[test]
fn parses_field_then_index() {
    assert_eq!(
        parse(".items[0]").unwrap(),
        Ast::pipe(Ast::Field("items".into()), Ast::Index(0))
    );
}

#[test]
fn parses_bracket_string_field() {
    assert_eq!(parse(r#".["a b"]"#).unwrap(), Ast::Field("a b".into()));
}

// Feature f030: jq's quoted field is the bracket form under another
// spelling, at the head of a path and at every later step.
#[test]
fn parses_quoted_field() {
    assert_eq!(parse(r#"."a.b""#).unwrap(), Ast::Field("a.b".into()));
    assert_eq!(parse(r#"."a.b""#).unwrap(), parse(r#".["a.b"]"#).unwrap());
    assert_eq!(parse(r#"."""#).unwrap(), Ast::Field(String::new()));
}

#[test]
fn parses_quoted_field_in_a_chain() {
    assert_eq!(
        parse(r#".labels."app.kubernetes.io/name""#).unwrap(),
        Ast::pipe(
            Ast::Field("labels".into()),
            Ast::Field("app.kubernetes.io/name".into())
        )
    );
    assert_eq!(
        parse(r#"."a.b".c[0]"#).unwrap(),
        Ast::pipe(
            Ast::pipe(Ast::Field("a.b".into()), Ast::Field("c".into())),
            Ast::Index(0)
        )
    );
    assert_eq!(
        parse(r#"to_entries[]."k.v""#).unwrap(),
        Ast::pipe(
            Ast::pipe(Ast::Builtin(Builtin::ToEntries), Ast::Iterate),
            Ast::Field("k.v".into())
        )
    );
}

#[test]
fn quoted_field_is_a_mutation_target() {
    assert_eq!(
        parse_program(r#"key(."a.b") = "c""#).unwrap(),
        Program::Mutate(Mutation::Assign {
            target: Target::Key(Ast::Field("a.b".into())),
            rhs: Rhs::Literal(Value::String("c".into())),
        })
    );
    assert_eq!(
        parse_program(r#"del(."a.b")"#).unwrap(),
        Program::Mutate(Mutation::Delete {
            target: Target::Value(Ast::Field("a.b".into())),
        })
    );
}

#[test]
fn a_dot_followed_by_a_literal_is_still_refused() {
    let err = parse(".a.5").unwrap_err();
    assert!(
        matches!(err, YqrError::Parse(ref m) if m.message.contains("quoted key")),
        "{err:?}"
    );
}

/// The span a parse error carries slices the offending token out of the
/// filter string — the contract an editor relies on to underline it.
// Feature f048.
#[test]
fn parse_error_span_slices_the_offending_token() {
    let filter = ".a = banana";
    let err = parse_program(filter).unwrap_err();
    let span = err.span().expect("a bare-identifier RHS has a span");
    assert_eq!(&filter[span], "banana", "{err:?}");

    let filter = ".a]";
    let err = parse_program(filter).unwrap_err();
    let span = err.span().expect("a trailing token has a span");
    assert_eq!(&filter[span], "]", "{err:?}");

    let filter = "swap(.xs; 0, 1)";
    let err = parse_program(filter).unwrap_err();
    let span = err.span().expect("the lexer locates the ','");
    assert_eq!(&filter[span], ",", "{err:?}");

    // Spans are byte ranges even when multi-byte characters precede the
    // offending token, so slicing never lands inside a character.
    let filter = r#"."käse" = banana"#;
    let err = parse_program(filter).unwrap_err();
    let span = err.span().expect("a bare-identifier RHS has a span");
    assert_eq!(&filter[span], "banana", "{err:?}");
}

/// A filter that ends too early is anchored at an empty range at the end
/// of the source, where the missing token would go.
// Feature f048.
#[test]
fn parse_error_at_end_of_input_has_an_empty_end_span() {
    let filter = ".a[";
    let err = parse_program(filter).unwrap_err();
    assert_eq!(err.span(), Some(3..3), "{err:?}");
}

#[test]
fn parses_pipe() {
    assert_eq!(
        parse(".a | .b").unwrap(),
        Ast::pipe(Ast::Field("a".into()), Ast::Field("b".into()))
    );
}

#[test]
fn parses_optional() {
    assert_eq!(parse(".a?").unwrap(), Ast::optional(Ast::Field("a".into())));
}

#[test]
fn parses_iterate_then_field() {
    assert_eq!(
        parse(".items[].name").unwrap(),
        Ast::pipe(
            Ast::pipe(Ast::Field("items".into()), Ast::Iterate),
            Ast::Field("name".into())
        )
    );
}

#[test]
fn rejects_non_dot_start() {
    assert!(matches!(parse("foo"), Err(YqrError::Parse(_))));
}

#[test]
fn rejects_trailing_garbage() {
    assert!(matches!(parse(".a]"), Err(YqrError::Parse(_))));
}

// -- Feature f006: mutation programs ---------------------------------------

#[test]
fn plain_query_is_a_query_program() {
    assert_eq!(
        parse_program(".a.b").unwrap(),
        Program::Query(Target::Value(Ast::pipe(
            Ast::Field("a".into()),
            Ast::Field("b".into())
        )))
    );
}

#[test]
fn parses_scalar_assignment() {
    assert_eq!(
        parse_program(".spec.replicas = 5").unwrap(),
        Program::Mutate(Mutation::Assign {
            target: Target::Value(Ast::pipe(
                Ast::Field("spec".into()),
                Ast::Field("replicas".into())
            )),
            rhs: Rhs::Literal(Value::Int(5)),
        })
    );
}

#[test]
fn parses_literal_rhs_variants() {
    let cases = [
        ("=1.5", Rhs::Literal(Value::Float(1.5))),
        (r#"= "hi""#, Rhs::Literal(Value::String("hi".into()))),
        ("= true", Rhs::Literal(Value::Bool(true))),
        ("= false", Rhs::Literal(Value::Bool(false))),
        ("= null", Rhs::Literal(Value::Null)),
    ];
    for (op, want) in cases {
        let src = format!(".a {op}");
        match parse_program(&src).unwrap() {
            Program::Mutate(Mutation::Assign { rhs, .. }) => assert_eq!(rhs, want),
            other => panic!("expected assign for {src:?}, got {other:?}"),
        }
    }
}

#[test]
fn parses_path_rhs() {
    assert_eq!(
        parse_program(".a = .b.c").unwrap(),
        Program::Mutate(Mutation::Assign {
            target: Target::Value(Ast::Field("a".into())),
            rhs: Rhs::Path(Ast::pipe(Ast::Field("b".into()), Ast::Field("c".into()))),
        })
    );
}

#[test]
fn parses_append() {
    assert_eq!(
        parse_program(".ports += 9090").unwrap(),
        Program::Mutate(Mutation::Append {
            path: Ast::Field("ports".into()),
            rhs: Rhs::Literal(Value::Int(9090)),
        })
    );
}

#[test]
fn parses_delete() {
    assert_eq!(
        parse_program("del(.metadata.labels)").unwrap(),
        Program::Mutate(Mutation::Delete {
            target: Target::Value(Ast::pipe(
                Ast::Field("metadata".into()),
                Ast::Field("labels".into())
            )),
        })
    );
}

#[test]
fn parses_a_computed_update() {
    assert_eq!(
        parse_program(".a |= (. + 1)").unwrap(),
        Program::Mutate(Mutation::Update {
            path: Ast::Field("a".into()),
            rhs: Ast::Binary(
                BinOp::Add,
                Box::new(Ast::Identity),
                Box::new(Ast::Literal(Value::Int(1)))
            ),
        })
    );
}

#[test]
fn arithmetic_precedence_and_grouping() {
    // `*` binds tighter than `+`; parentheses beat both.
    assert_eq!(
        parse("1 + 2 * 3").unwrap(),
        Ast::Binary(
            BinOp::Add,
            Box::new(Ast::Literal(Value::Int(1))),
            Box::new(Ast::Binary(
                BinOp::Mul,
                Box::new(Ast::Literal(Value::Int(2))),
                Box::new(Ast::Literal(Value::Int(3)))
            ))
        )
    );
    assert_eq!(
        parse("(1 + 2) * 3").unwrap(),
        Ast::Binary(
            BinOp::Mul,
            Box::new(Ast::Binary(
                BinOp::Add,
                Box::new(Ast::Literal(Value::Int(1))),
                Box::new(Ast::Literal(Value::Int(2)))
            )),
            Box::new(Ast::Literal(Value::Int(3)))
        )
    );
}

#[test]
fn pipe_binds_looser_than_arithmetic() {
    // `.a + 1 | .` is `(.a + 1) | .`, matching jq.
    let got = parse(".a + 1 | .").unwrap();
    let Ast::Pipe(lhs, _) = &got else {
        panic!("expected a pipe at the top: {got:?}");
    };
    assert!(
        matches!(**lhs, Ast::Binary(..)),
        "the arithmetic must bind tighter than the pipe: {got:?}"
    );
}

#[test]
fn bare_identifier_rhs_is_rejected() {
    assert!(matches!(parse_program(".a = foo"), Err(YqrError::Parse(_))));
}

#[test]
fn read_only_parse_rejects_a_mutation() {
    // The query-only `parse` entry point must refuse a mutating filter.
    assert!(matches!(parse(".a = 5"), Err(YqrError::Parse(_))));
    assert!(matches!(parse("del(.a)"), Err(YqrError::Parse(_))));
}

#[test]
fn del_without_parens_is_a_parse_error() {
    // `del` alone (no call parens) is not a valid query start.
    assert!(parse_program("del").is_err());
}

// -- Feature f007: the key selector -----------------------------------

#[test]
fn parses_a_key_read() {
    assert_eq!(
        parse_program("key(.metadata.name)").unwrap(),
        Program::Query(Target::Key(Ast::pipe(
            Ast::Field("metadata".into()),
            Ast::Field("name".into())
        )))
    );
}

#[test]
fn parses_a_key_rename() {
    let Program::Mutate(Mutation::Assign { target, rhs }) =
        parse_program("key(.a) = \"b\"").unwrap()
    else {
        panic!("expected an assignment");
    };
    assert_eq!(target, Target::Key(Ast::Field("a".into())));
    assert_eq!(rhs, Rhs::Literal(Value::String("b".into())));
}

#[test]
fn selector_words_stay_field_accesses_outside_function_position() {
    // The whole "no new reserved words" claim, checked against the words
    // this grammar actually spends. `swap` and `move` are ordinary YAML
    // field names, so a regression here would break read-only queries.
    for word in [
        "key",
        "del",
        "swap",
        "move",
        "line_comment",
        "head_comment",
        "foot_comment",
    ] {
        let src = format!(".{word}");
        assert_eq!(
            parse_program(&src).unwrap(),
            Program::Query(Target::Value(Ast::Field(word.into()))),
            "{src} should parse as a field access"
        );
    }
}

#[test]
fn a_key_selector_iterates_like_the_path_it_wraps() {
    assert_eq!(
        parse_program("key(.items[])").unwrap(),
        Program::Query(Target::Key(Ast::pipe(
            Ast::Field("items".into()),
            Ast::Iterate
        )))
    );
}

#[test]
fn del_still_takes_a_pipeline() {
    // a002 2.3 keeps `del`'s existing argument rather than narrowing it to
    // a bare path; this is the regression that narrowing would cause.
    let Program::Mutate(Mutation::Delete { target }) = parse_program("del(.a | .b)").unwrap()
    else {
        panic!("expected a delete");
    };
    assert_eq!(
        target,
        Target::Value(Ast::pipe(Ast::Field("a".into()), Ast::Field("b".into())))
    );
}

#[test]
fn del_of_a_key_is_refused_with_a_reason() {
    let err = parse_program("del(key(.a))").unwrap_err();
    let text = format!("{err}");
    assert!(text.contains("cannot outlive its entry"), "got: {text}");
    assert!(
        text.contains("key(<path>) = "),
        "should suggest rename: {text}"
    );
}

#[test]
fn append_to_a_key_is_refused_with_a_reason() {
    let err = parse_program("key(.a) += 1").unwrap_err();
    assert!(
        format!("{err}").contains("does not name a sequence"),
        "got: {err}"
    );
}

#[test]
fn parses_the_comment_selectors() {
    assert_eq!(
        parse_program("line_comment(.a)").unwrap(),
        Program::Query(Target::LineComment(Ast::Field("a".into())))
    );
    assert_eq!(
        parse_program("head_comment(.a)").unwrap(),
        Program::Query(Target::HeadComment(Ast::Field("a".into())))
    );
}

#[test]
fn foot_comment_is_parsed_then_refused_with_a_reason() {
    // The word is in the grammar precisely so this message exists rather
    // than an unexpected-token report (a002 §8).
    for src in [
        "foot_comment(.a)",
        "foot_comment(.a) = \"x\"",
        "del(foot_comment(.a))",
    ] {
        let text = format!("{}", parse_program(src).unwrap_err());
        assert!(text.contains("foot_comment"), "should name it: {text}");
        assert!(text.contains("head_comment"), "should redirect: {text}");
    }
}

#[test]
fn del_composes_with_the_comment_selectors() {
    let Program::Mutate(Mutation::Delete { target }) =
        parse_program("del(line_comment(.a))").unwrap()
    else {
        panic!("expected a delete");
    };
    assert_eq!(target, Target::LineComment(Ast::Field("a".into())));
}

#[test]
fn the_read_only_entry_point_rejects_a_selector() {
    // `parse` feeds the classic pipeline, which has no document bytes.
    let err = parse("key(.a)").unwrap_err();
    assert!(format!("{err}").contains("normalize"), "got: {err}");
}

// -- Feature f007: the reorder verbs -----------------------------------

#[test]
fn parses_swap_and_move() {
    assert_eq!(
        parse_program("swap(.spec.containers; 0; 2)").unwrap(),
        Program::Mutate(Mutation::Reorder {
            path: Ast::pipe(Ast::Field("spec".into()), Ast::Field("containers".into())),
            op: ReorderOp::Swap,
            from: 0,
            to: 2,
        })
    );
    assert_eq!(
        parse_program("move(.; 1; 0)").unwrap(),
        Program::Mutate(Mutation::Reorder {
            path: Ast::Identity,
            op: ReorderOp::Move,
            from: 1,
            to: 0,
        })
    );
}

#[test]
fn a_reorder_index_may_be_negative() {
    let Program::Mutate(Mutation::Reorder { from, to, .. }) =
        parse_program("swap(.xs; 0; -1)").unwrap()
    else {
        panic!("expected a reorder");
    };
    assert_eq!((from, to), (0, -1));
}

#[test]
fn a_missing_separator_names_the_whole_form() {
    // `;` is the one token of this grammar a jq or yq user has no reason
    // to expect, so the message spells the form rather than the token.
    let text = format!("{}", parse_program("swap(.xs 0 1)").unwrap_err());
    assert!(text.contains("swap(<path>; i; j)"), "got: {text}");
    // `move`'s arguments are not interchangeable, and the message says so.
    let text = format!("{}", parse_program("move(.xs 0 1)").unwrap_err());
    assert!(text.contains("move(<path>; from; to)"), "got: {text}");
}

#[test]
fn a_reorder_index_must_be_an_integer() {
    for src in [
        "swap(.xs; \"a\"; 1)",
        "swap(.xs; 0; .y)",
        "move(.xs; 1.5; 0)",
    ] {
        let text = format!("{}", parse_program(src).unwrap_err());
        assert!(
            text.contains("two integer indices"),
            "{src} should say so: {text}"
        );
    }
}

#[test]
fn a_reorder_is_not_a_target() {
    // There is no node to name, so a reorder composes with neither `del`
    // nor `=`; both are parse errors rather than silently accepted forms.
    assert!(parse_program("del(swap(.xs; 0; 1))").is_err());
    assert!(parse_program("swap(.xs; 0; 1) = 5").is_err());
}

#[test]
fn the_read_only_entry_point_rejects_a_reorder() {
    let text = format!("{}", parse("swap(.xs; 0; 1)").unwrap_err());
    assert!(text.contains("mutation"), "got: {text}");
}

// -- Feature f017: the to_entries builtin ------------------------------

#[test]
fn parses_a_builtin_in_term_position() {
    assert_eq!(
        parse(".m | to_entries").unwrap(),
        Ast::pipe(Ast::Field("m".into()), Ast::Builtin(Builtin::ToEntries))
    );
}

#[test]
fn a_builtin_takes_a_chain_the_way_a_path_does() {
    // `to_entries[]` must be "the builtin, then iterate" rather than a
    // parse error, and `to_entries[].key` must keep going.
    assert_eq!(
        parse("to_entries[]").unwrap(),
        Ast::pipe(Ast::Builtin(Builtin::ToEntries), Ast::Iterate)
    );
    assert_eq!(
        parse("to_entries[].key").unwrap(),
        Ast::pipe(
            Ast::pipe(Ast::Builtin(Builtin::ToEntries), Ast::Iterate),
            Ast::Field("key".into())
        )
    );
    assert_eq!(
        parse("to_entries[0]").unwrap(),
        Ast::pipe(Ast::Builtin(Builtin::ToEntries), Ast::Index(0))
    );
}

#[test]
fn a_builtin_takes_the_optional_suffix() {
    assert_eq!(
        parse("to_entries?").unwrap(),
        Ast::optional(Ast::Builtin(Builtin::ToEntries))
    );
}

#[test]
fn to_entries_costs_no_reserved_word() {
    // The whole reason the word is safe: a path starts with '.', so an
    // identifier there can only be a field name. This test fails the day
    // someone reserves it.
    assert_eq!(
        parse(".to_entries").unwrap(),
        Ast::Field("to_entries".into())
    );
    assert_eq!(
        parse(".a.to_entries").unwrap(),
        Ast::pipe(Ast::Field("a".into()), Ast::Field("to_entries".into()))
    );
    assert_eq!(
        parse(".[\"to_entries\"]").unwrap(),
        Ast::Field("to_entries".into())
    );
}

#[test]
fn a_builtin_called_with_parentheses_is_told_it_takes_none() {
    let err = parse("to_entries(.m)").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("takes no arguments"), "{msg}");
    assert!(
        msg.contains(".m | to_entries") || msg.contains("| to_entries"),
        "{msg}"
    );
}

#[test]
fn every_write_form_refuses_a_builtin_at_parse() {
    for (filter, op) in [
        (".m | to_entries = 1", "'='"),
        (".m | to_entries += 1", "'+='"),
        ("del(.m | to_entries)", "'del'"),
        ("to_entries = 1", "'='"),
        // The reorder verbs are the fourth site, and the one where a
        // missing guard hides: an unresolvable reorder path is an
        // *absent* one, which the driver leaves alone at exit 0, so the
        // failure mode is a success code for an edit that did nothing.
        ("swap(.m | to_entries; 0; 1)", "'swap'"),
        ("move(to_entries; 0; 1)", "'move'"),
    ] {
        let err = parse_program(filter).unwrap_err();
        assert!(
            matches!(err, YqrError::Parse(_)),
            "{filter}: expected a parse error, got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("to_entries") && msg.contains(op),
            "{filter}: message should name the builtin and {op}, got {msg}"
        );
        assert!(
            msg.contains("nothing to write back to"),
            "{filter}: message should say why, got {msg}"
        );
    }
}

#[test]
fn a_builtin_query_is_a_query_program() {
    assert_eq!(
        parse_program(".m | to_entries").unwrap(),
        Program::Query(Target::Value(Ast::pipe(
            Ast::Field("m".into()),
            Ast::Builtin(Builtin::ToEntries)
        )))
    );
}
