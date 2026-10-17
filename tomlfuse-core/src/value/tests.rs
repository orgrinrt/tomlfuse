use super::*;
use std::f64::consts::PI;

/// Binds the value written as `value = <text>`.
fn bound(text: &str) -> Result<(Ty, Lit), Problem> {
    bound_at(text, None)
}

/// The same at a width.
fn bound_at(text: &str, width: Option<&str>) -> Result<(Ty, Lit), Problem> {
    let parsed: Value = format!("value = {text}").parse().unwrap();
    bind_value(
        "value",
        parsed.get("value").unwrap(),
        width.map(|w| Width::parse(w).unwrap()),
    )
}

const INT: Ty = Ty::Int(IntTy::I64);

fn slice(of: Ty) -> Ty {
    Ty::Slice(Box::new(of))
}

#[test]
fn test_string_value_conversion() {
    let (ty, val) = bind_value("k", &Value::String("test".to_string()), None).unwrap();
    assert_eq!(ty, Ty::Str);
    assert_eq!(val, Lit::Str("test".into()));
}

#[test]
fn test_numeric_values() {
    assert_eq!(
        bind_value("k", &Value::Integer(42), None).unwrap(),
        (Ty::Int(IntTy::I64), Lit::Int(42))
    );
    assert_eq!(
        bind_value("k", &Value::Float(PI), None).unwrap(),
        (Ty::Float(FloatTy::F64), Lit::Float(PI))
    );
}

#[test]
fn test_boolean_value() {
    assert_eq!(bound("true").unwrap(), (Ty::Bool, Lit::Bool(true)));
    assert_eq!(bound("false").unwrap().1, Lit::Bool(false));
}

#[test]
fn test_homogeneous_array() {
    let (ty, val) = bound(r#"["a", "b"]"#).unwrap();
    assert_eq!(ty, slice(Ty::Str));
    assert_eq!(
        val,
        Lit::List(vec![Lit::Str("a".into()), Lit::Str("b".into())])
    );
}

#[test]
fn test_empty_array() {
    let (ty, val) = bound("[]").unwrap();
    assert_eq!(ty, slice(Ty::Str));
    assert_eq!(val, Lit::List(vec![]));
}

#[test]
fn an_array_of_arrays_is_a_slice_of_slices() {
    let (ty, _) = bound("[[1, 2], [3]]").unwrap();
    assert_eq!(ty, slice(slice(INT)));
}

#[test]
fn an_empty_array_takes_the_kind_of_the_arrays_beside_it() {
    // Whichever side it is on. Taking the first element's type for the lot typed the
    // first of these as arrays of strings, and the integers inside it would not compile.
    assert_eq!(bound("[[], [1, 2]]").unwrap().0, slice(slice(INT)));
    assert_eq!(bound("[[1, 2], []]").unwrap().0, slice(slice(INT)));
    assert_eq!(bound("[[], [[1]]]").unwrap().0, slice(slice(slice(INT))));
    // With nothing beside it to say, it stays an array of strings, as it always was.
    assert_eq!(bound("[[], []]").unwrap().0, slice(slice(Ty::Str)));
}

#[test]
fn a_mixed_array_is_refused_naming_the_key_and_the_kinds() {
    let err = bound(r#"[1, "two"]"#).unwrap_err();
    assert_eq!(err.key, "value");
    assert!(err.reason.contains("an integer and a string"), "{err}");
    // Two numbers of different kinds are still two kinds.
    assert!(bound("[1, 2.5]")
        .unwrap_err()
        .reason
        .contains("an integer and a float"));
}

#[test]
fn a_mix_inside_an_inner_array_is_refused_too() {
    let err = bound(r#"[[1], ["a"]]"#).unwrap_err();
    assert!(err.reason.contains("an integer and a string"), "{err}");
    let err = bound("[[1], 2]").unwrap_err();
    assert!(err.reason.contains("an array and an integer"), "{err}");
}

#[test]
fn a_table_in_an_array_is_refused_naming_the_element() {
    let err = bound(r#"[{ name = "a" }, { name = "b" }]"#).unwrap_err();
    assert_eq!(err.key, "value[0]");
    assert!(err.reason.contains("table inside an array"), "{err}");
}

#[test]
fn a_datetime_is_refused_wherever_it_is() {
    let err = bound("2026-10-17T09:30:00Z").unwrap_err();
    assert_eq!(err.key, "value");
    assert!(err.reason.contains("datetime"), "{err}");

    let err = bound("[[1], [2026-10-17T09:30:00Z]]").unwrap_err();
    assert_eq!(err.key, "value[1][0]");
}

#[test]
fn a_problem_reads_as_a_sentence_about_its_key() {
    let err = bound("2026-10-17T09:30:00Z").unwrap_err();
    assert_eq!(
        err.to_string(),
        "tomlfuse: `value` is a datetime, which has no type to bind as. Write it as a \
         string if its text is what is wanted."
    );
}

#[test]
fn an_integer_binds_at_the_width_named() {
    let (ty, val) = bound_at("17", Some("u8")).unwrap();
    assert_eq!(ty, Ty::Int(IntTy::U8));
    assert_eq!(val, Lit::Int(17));
    assert_eq!(bound_at("-12", Some("i16")).unwrap().0, Ty::Int(IntTy::I16));
}

#[test]
fn an_integer_at_the_edge_of_its_width_fits_and_one_past_it_does_not() {
    assert!(bound_at("255", Some("u8")).is_ok());
    assert!(bound_at("0", Some("u8")).is_ok());
    assert!(bound_at("-128", Some("i8")).is_ok());
    assert!(bound_at("127", Some("i8")).is_ok());
    assert!(bound_at("9223372036854775807", Some("u64")).is_ok());

    for (text, width, bounds) in [
        ("256", "u8", "0 to 255"),
        ("-1", "u8", "0 to 255"),
        ("128", "i8", "-128 to 127"),
        ("-129", "i8", "-128 to 127"),
        ("65536", "u16", "0 to 65535"),
        ("-1", "u64", "0 to 18446744073709551615"),
    ] {
        let err = bound_at(text, Some(width)).unwrap_err();
        assert_eq!(err.key, "value");
        assert_eq!(
            err.reason,
            format!("is {text}, which does not fit {width} ({bounds})."),
            "{text} as {width}"
        );
    }
}

#[test]
fn a_float_binds_at_the_width_named_and_a_float_too_large_does_not_fit() {
    assert_eq!(
        bound_at("0.5", Some("f32")).unwrap().0,
        Ty::Float(FloatTy::F32)
    );
    assert_eq!(
        bound_at("1e300", Some("f64")).unwrap().0,
        Ty::Float(FloatTy::F64)
    );

    let err = bound_at("1e39", Some("f32")).unwrap_err();
    assert_eq!(
        err.reason,
        "is 1e39, which does not fit f32 (magnitude up to 3.4028235e38)."
    );
    assert!(bound_at("-1e39", Some("f32")).is_err());
}

#[test]
fn the_largest_f32_fits_however_it_is_written_and_the_first_float_past_it_does_not() {
    // The limit the message names, written the way the message writes it. It is not
    // exactly `f32::MAX` as an `f64`, which is why comparing against that refused it.
    let max = format!("{:e}", f32::MAX);
    assert_eq!(max, "3.4028235e38");
    assert!(bound_at(&max, Some("f32")).is_ok(), "{max}");
    assert!(bound_at(&format!("-{max}"), Some("f32")).is_ok());
    assert!(bound_at("3.4028234e38", Some("f32")).is_ok());
    // Exactly `f32::MAX`, from the other direction.
    assert!(bound_at(&format!("{:e}", f64::from(f32::MAX)), Some("f32")).is_ok());

    // Halfway to the next power of two is where narrowing stops rounding down to
    // `f32::MAX` and starts rounding up to infinity: below it fits, at it does not.
    let halfway = f64::from(f32::MAX) + 2f64.powi(103);
    assert!((halfway as f32).is_infinite());
    assert_eq!(halfway.next_down() as f32, f32::MAX);
    let below = format!("{:e}", halfway.next_down());
    let at = format!("{:e}", halfway);
    assert!(bound_at(&below, Some("f32")).is_ok(), "{below}");
    assert!(
        bound_at(&format!("-{below}"), Some("f32")).is_ok(),
        "-{below}"
    );
    let err = bound_at(&at, Some("f32")).unwrap_err();
    assert!(err.reason.contains("does not fit f32"), "{at}: {err}");
    assert!(bound_at(&format!("-{at}"), Some("f32")).is_err(), "-{at}");

    // `f64` has no width above it to overflow.
    assert!(bound_at(&format!("{:e}", f64::MAX), Some("f64")).is_ok());
}

#[test]
fn a_float_too_small_for_f32_is_refused_instead_of_becoming_zero() {
    let smallest = f32::from_bits(1);
    let text = format!("{:e}", smallest);
    assert!(bound_at(&text, Some("f32")).is_ok(), "{text}");
    assert!(bound_at("1.5e-45", Some("f32")).is_ok());

    // Half the smallest magnitude ties to even, which is zero. Anything above it still
    // reads as the smallest, and is a value rather than a loss.
    let half = f64::from(smallest) / 2.0;
    assert!(bound_at(&format!("{:e}", half.next_up()), Some("f32")).is_ok());
    let err = bound_at(&format!("{:e}", half), Some("f32")).unwrap_err();
    assert!(err.reason.contains("too small for f32"), "{err}");
    assert!(err.reason.contains("1e-45"), "{err}");
    assert!(bound_at("-1e-46", Some("f32")).is_err());

    // Zero is zero, whichever way round, and `f64` reaches far lower.
    assert!(bound_at("0.0", Some("f32")).is_ok());
    assert!(bound_at("-0.0", Some("f32")).is_ok());
    assert!(bound_at("1e-46", Some("f64")).is_ok());
    assert!(bound_at("1e-300", Some("f64")).is_ok());
}

#[test]
fn infinity_and_nan_are_refused_naming_the_key_with_or_without_a_width() {
    for (text, shown) in [("inf", "inf"), ("-inf", "-inf"), ("nan", "NaN"), ("-nan", "NaN")] {
        for width in [None, Some("f32"), Some("f64")] {
            let err = bound_at(text, width).unwrap_err();
            assert_eq!(err.key, "value", "{text} at {width:?}");
            assert!(
                err.reason.starts_with(&format!("is {shown}, which")),
                "{err}"
            );
        }
    }
    let err = bound_at("[1.0, inf]", Some("f32")).unwrap_err();
    assert_eq!(err.key, "value[1]");
    assert_eq!(bound("[1.0, nan]").unwrap_err().key, "value[1]");
}

#[test]
fn a_width_does_not_convert_between_integers_and_floats() {
    let err = bound_at("0.5", Some("u8")).unwrap_err();
    assert_eq!(err.reason, "is a float, and `u8` is an integer type.");
    let err = bound_at("3", Some("f32")).unwrap_err();
    assert!(
        err.reason
            .starts_with("is an integer, and `f32` is a float type."),
        "{err}"
    );
}

#[test]
fn a_width_on_anything_but_a_number_is_refused() {
    let err = bound_at(r#""text""#, Some("u32")).unwrap_err();
    assert_eq!(
        err.reason,
        "is a string, and the width `u32` applies to integers and floats only."
    );
    let err = bound_at("true", Some("u32")).unwrap_err();
    assert!(err.reason.starts_with("is a boolean"), "{err}");
}

fn array(of: Ty, len: usize) -> Ty {
    Ty::Array(Box::new(of), len)
}

#[test]
fn the_width_reaches_every_number_in_an_array_and_names_the_one_that_does_not_fit() {
    let (ty, _) = bound_at("[[1, 2], [3, 4]]", Some("u8")).unwrap();
    assert_eq!(ty, array(array(Ty::Int(IntTy::U8), 2), 2));

    let err = bound_at("[[1, 2], [3, 300]]", Some("u8")).unwrap_err();
    assert_eq!(err.key, "value[1][1]");
    assert!(err.reason.contains("does not fit u8"), "{err}");
}

#[test]
fn an_array_with_a_width_is_fixed_at_the_length_the_file_gave_it() {
    let (ty, val) = bound_at("[1, 2, 3]", Some("u8")).unwrap();
    assert_eq!(ty, array(Ty::Int(IntTy::U8), 3));
    assert_eq!(val, Lit::List(vec![Lit::Int(1), Lit::Int(2), Lit::Int(3)]));

    // Both lengths, the outer and the inner.
    let (ty, _) = bound_at("[[1, 2, 3], [4, 5, 6]]", Some("u8")).unwrap();
    assert_eq!(ty, array(array(Ty::Int(IntTy::U8), 3), 2));

    // Without a width the same value is the slices it always was.
    let (ty, _) = bound("[[1, 2, 3], [4, 5, 6]]").unwrap();
    assert_eq!(ty, slice(slice(INT)));
}

#[test]
fn an_empty_array_with_a_width_is_an_empty_array_of_the_width() {
    assert_eq!(
        bound_at("[]", Some("u8")).unwrap().0,
        array(Ty::Int(IntTy::U8), 0)
    );
    assert_eq!(
        bound_at("[[], []]", Some("f32")).unwrap().0,
        array(array(Ty::Float(FloatTy::F32), 0), 2)
    );
}

#[test]
fn rows_of_different_lengths_cannot_be_a_fixed_array() {
    let err = bound_at("[[1, 2, 3], [4, 5]]", Some("u8")).unwrap_err();
    assert_eq!(err.key, "value");
    assert!(
        err.reason
            .starts_with("holds arrays of different lengths, 3 and 2 (at [1])."),
        "{err}"
    );

    // An empty row is a row of another length, not a gap to be filled in.
    assert!(bound_at("[[], [1]]", Some("u8")).is_err());
    // And the difference may be further in, where the message names the real place.
    let err = bound_at("[[[1, 2]], [[1, 2, 3]]]", Some("u8")).unwrap_err();
    assert!(
        err.reason
            .starts_with("holds arrays of different lengths, 2 and 3 (at [1][0])."),
        "{err}"
    );
    let err = bound_at("[[[1]], [[1]], [[1, 2]]]", Some("u8")).unwrap_err();
    assert!(err.reason.contains("1 and 2 (at [2][0])."), "{err}");
    let err = bound_at("[[[[1]]], [[[1, 2]]]]", Some("u8")).unwrap_err();
    assert!(err.reason.contains("1 and 2 (at [1][0][0])."), "{err}");
    // The same rows are fine as slices.
    assert!(bound("[[1, 2, 3], [4, 5]]").is_ok());
}
