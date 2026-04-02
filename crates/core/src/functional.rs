use crate::Result;

pub type FallibleTransform<T, U> = fn(T) -> Result<U>;

pub type Validator<T> = fn(&T) -> Result<()>;

pub fn validate_all<T, F>(item: &T, validators: &[F]) -> Result<()>
where
    F: Fn(&T) -> Result<()>,
{
    validators
        .iter()
        .try_fold((), |(), validator| validator(item))
}

pub fn compose_result<T, U, V>(
    f: impl Fn(T) -> Result<U>,
    g: impl Fn(U) -> Result<V> + Clone,
) -> impl Fn(T) -> Result<V> {
    move |x| f(x).and_then(g.clone())
}

pub fn apply_transforms<T, F>(item: T, transforms: &[F]) -> Result<T>
where
    F: Fn(T) -> Result<T>,
{
    transforms
        .iter()
        .try_fold(item, |acc, transform| transform(acc))
}

pub fn group_by<T, K, F>(items: Vec<T>, key_fn: F) -> im::HashMap<K, Vec<T>>
where
    K: std::hash::Hash + Eq + Clone,
    T: Clone,
    F: Fn(&T) -> K,
{
    items.into_iter().fold(im::HashMap::new(), |mut map, item| {
        let key = key_fn(&item);
        #[allow(clippy::unnecessary_option_map_or_else)]
        let mut group = map
            .get(&key)
            .map_or_else(Vec::new, std::clone::Clone::clone);
        group.push(item);
        map.insert(key, group);
        map
    })
}

pub fn partition<T, F>(items: Vec<T>, predicate: F) -> (Vec<T>, Vec<T>)
where
    F: Fn(&T) -> bool,
{
    items.into_iter().partition(predicate)
}

pub fn fold_result<T, U, F>(items: Vec<T>, init: U, f: F) -> Result<U>
where
    F: Fn(U, T) -> Result<U>,
{
    items.into_iter().try_fold(init, f)
}

pub fn map_result<T, U, F>(items: Vec<T>, f: F) -> Result<Vec<U>>
where
    F: Fn(T) -> Result<U>,
{
    items.into_iter().map(f).collect()
}

pub fn filter_result<T, F>(items: Vec<T>, f: F) -> Result<Vec<T>>
where
    F: Fn(&T) -> Result<bool>,
{
    items.into_iter().try_fold(Vec::new(), |mut acc, item| {
        f(&item).map(|keep| {
            if keep {
                acc.push(item);
            }
            acc
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    fn is_positive(n: &i32) -> Result<()> {
        if *n > 0 {
            Ok(())
        } else {
            Err(Error::validation_error("not positive"))
        }
    }

    fn is_even(n: &i32) -> Result<()> {
        if n % 2 == 0 {
            Ok(())
        } else {
            Err(Error::validation_error("not even"))
        }
    }

    #[test]
    fn test_validate_all_success() {
        let validators: Vec<fn(&i32) -> Result<()>> = vec![is_positive, is_even];
        assert!(validate_all(&4, &validators).is_ok());
    }

    #[test]
    fn test_validate_all_failure() {
        let validators: Vec<fn(&i32) -> Result<()>> = vec![is_positive, is_even];
        assert!(validate_all(&3, &validators).is_err());
    }

    #[test]
    fn test_compose_result() {
        let double = |x: i32| -> Result<i32> { Ok(x * 2) };
        let add_one = |x: i32| -> Result<i32> { Ok(x + 1) };
        let composed = compose_result(double, add_one);

        let result = composed(5);
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("composition failed: {e}"),
        };
        assert_eq!(value, 11);
    }

    #[test]
    fn test_group_by() {
        let items = vec![("a", 1), ("b", 2), ("a", 3), ("b", 4)];
        let grouped = group_by(items, |(key, _)| *key);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get("a").map_or(0, Vec::len), 2);
        assert_eq!(grouped.get("b").map_or(0, Vec::len), 2);
    }

    #[test]
    fn test_partition() {
        let items = vec![1, 2, 3, 4, 5, 6];
        let (even, odd) = partition(items, |x| x % 2 == 0);

        assert_eq!(even, vec![2, 4, 6]);
        assert_eq!(odd, vec![1, 3, 5]);
    }

    #[test]
    fn test_fold_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = fold_result(items, 0, |acc, x| Ok(acc + x));
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("fold failed: {e}"),
        };
        assert_eq!(value, 15);
    }

    #[test]
    fn test_map_result() {
        let items = vec![1, 2, 3];
        let result = map_result(items, |x| Ok(x * 2));
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("map failed: {e}"),
        };
        assert_eq!(value, vec![2, 4, 6]);
    }

    #[test]
    fn test_filter_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = filter_result(items, |x| Ok(x % 2 == 0));
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("filter failed: {e}"),
        };
        assert_eq!(value, vec![2, 4]);
    }

    // ── validate_all additional cases ────────────────────────────────────────

    #[test]
    fn test_validate_all_empty_validators() {
        // No validators = always ok
        let empty: Vec<fn(&i32) -> Result<()>> = vec![];
        assert!(validate_all(&42, &empty).is_ok());
    }

    #[test]
    fn test_validate_all_single_validator_passes() {
        let validators: Vec<fn(&i32) -> Result<()>> = vec![is_positive];
        assert!(validate_all(&5, &validators).is_ok());
    }

    #[test]
    fn test_validate_all_single_validator_fails() {
        let validators: Vec<fn(&i32) -> Result<()>> = vec![is_positive];
        assert!(validate_all(&-1, &validators).is_err());
    }

    #[test]
    fn test_validate_all_stops_at_first_failure() {
        let validators: Vec<fn(&i32) -> Result<()>> = vec![is_positive, is_even];
        // -3 fails is_positive, so is_even should not be checked
        let result = validate_all(&-3, &validators);
        let err_msg = result.expect_err("should fail").to_string();
        assert!(err_msg.contains("not positive"));
    }

    // ── compose_result additional cases ──────────────────────────────────────

    #[test]
    fn test_compose_result_first_fails() {
        let fail = |x: i32| -> Result<i32> { Err(Error::validation_error(format!("fail {x}"))) };
        let add_one = |x: i32| -> Result<i32> { Ok(x + 1) };
        let composed = compose_result(fail, add_one);

        assert!(composed(5).is_err());
    }

    #[test]
    fn test_compose_result_second_fails() {
        let ok = |x: i32| -> Result<i32> { Ok(x) };
        let fail = |_x: i32| -> Result<i32> { Err(Error::validation_error("second fails")) };
        let composed = compose_result(ok, fail);

        assert!(composed(5).is_err());
    }

    #[test]
    fn test_compose_result_triple_chain() {
        let add = |x: i32| -> Result<i32> { Ok(x + 1) };
        let double = |x: i32| -> Result<i32> { Ok(x * 2) };
        let composed12 = compose_result(add, double);
        let composed123 = compose_result(composed12, add);

        let result = composed123(3); // (3+1)*2+1 = 9
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("composition failed: {e}"),
        };
        assert_eq!(value, 9);
    }

    #[test]
    fn test_compose_result_identity() {
        let identity = |x: i32| -> Result<i32> { Ok(x) };
        let add_one = |x: i32| -> Result<i32> { Ok(x + 1) };
        let composed = compose_result(identity, add_one);

        let result = composed(10);
        let value = match result {
            Ok(v) => v,
            Err(e) => panic!("composition failed: {e}"),
        };
        assert_eq!(value, 11);
    }

    // ── apply_transforms ─────────────────────────────────────────────────────

    #[test]
    fn test_apply_transforms_empty() {
        let empty: Vec<fn(i32) -> Result<i32>> = vec![];
        let result = apply_transforms(5, &empty).expect("should succeed");
        assert_eq!(result, 5);
    }

    #[test]
    fn test_apply_transforms_single() {
        let transforms: Vec<fn(i32) -> Result<i32>> = vec![|x| Ok(x * 2)];
        let result = apply_transforms(3, &transforms).expect("should succeed");
        assert_eq!(result, 6);
    }

    #[test]
    fn test_apply_transforms_chain() {
        let transforms: Vec<fn(i32) -> Result<i32>> = vec![|x| Ok(x + 1), |x| Ok(x * 2), |x| Ok(x - 3)];
        // ((5 + 1) * 2) - 3 = 9
        let result = apply_transforms(5, &transforms).expect("should succeed");
        assert_eq!(result, 9);
    }

    #[test]
    fn test_apply_transforms_fails_mid_chain() {
        let transforms: Vec<fn(i32) -> Result<i32>> = vec![
            |x| Ok(x + 1),
            |_x| Err(Error::validation_error("mid-chain failure")),
            |x| Ok(x * 2),
        ];
        assert!(apply_transforms(5, &transforms).is_err());
    }

    // ── group_by additional cases ────────────────────────────────────────────

    #[test]
    fn test_group_by_empty() {
        let grouped = group_by(Vec::<(&str, i32)>::new(), |(key, _)| *key);
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_group_by_single_key() {
        let items = vec![1, 2, 3];
        let grouped = group_by(items, |_: &i32| "same");
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped.get("same").map_or(0, Vec::len), 3);
    }

    #[test]
    fn test_group_by_all_distinct_keys() {
        let items = vec![1, 2, 3];
        let grouped = group_by(items, |x| *x);
        assert_eq!(grouped.len(), 3);
        for i in 1..=3 {
            assert_eq!(grouped.get(&i).map_or(0, Vec::len), 1);
        }
    }

    #[test]
    fn test_group_by_preserves_values() {
        let items = vec![("a", 10), ("a", 20), ("b", 30)];
        let grouped = group_by(items, |(key, _)| *key);
        let a_values = grouped.get("a").expect("should have 'a'");
        assert_eq!(a_values, &[("a", 10), ("a", 20)]);
    }

    // ── partition additional cases ───────────────────────────────────────────

    #[test]
    fn test_partition_empty() {
        let (a, b) = partition::<i32, _>(vec![], |x| *x > 0);
        assert!(a.is_empty());
        assert!(b.is_empty());
    }

    #[test]
    fn test_partition_all_match() {
        let (a, b) = partition(vec![2, 4, 6], |x| x % 2 == 0);
        assert_eq!(a, vec![2, 4, 6]);
        assert!(b.is_empty());
    }

    #[test]
    fn test_partition_none_match() {
        let (a, b) = partition(vec![1, 3, 5], |x| x % 2 == 0);
        assert!(a.is_empty());
        assert_eq!(b, vec![1, 3, 5]);
    }

    // ── fold_result additional cases ─────────────────────────────────────────

    #[test]
    fn test_fold_result_empty() {
        let result = fold_result::<i32, i32, _>(vec![], 0, |acc, x| Ok(acc + x));
        assert_eq!(result.expect("ok"), 0);
    }

    #[test]
    fn test_fold_result_string_concat() {
        let items = vec!["hello", " ", "world"];
        let result = fold_result(items, String::new(), |mut acc, x| {
            acc.push_str(x);
            Ok(acc)
        });
        assert_eq!(result.expect("ok"), "hello world");
    }

    #[test]
    fn test_fold_result_fails() {
        let items = vec![1, 2, 3];
        let result = fold_result(items, 0, |_acc, x| {
            if x == 2 {
                Err(Error::validation_error("found 2"))
            } else {
                Ok(x)
            }
        });
        assert!(result.is_err());
    }

    // ── map_result additional cases ──────────────────────────────────────────

    #[test]
    fn test_map_result_empty() {
        let result = map_result::<i32, i32, _>(vec![], |x| Ok(x * 2));
        assert_eq!(result.expect("ok"), Vec::<i32>::new());
    }

    #[test]
    fn test_map_result_type_conversion() {
        let items = vec![1, 2, 3];
        let result = map_result(items, |x| Ok(x.to_string()));
        assert_eq!(result.expect("ok"), vec!["1", "2", "3"]);
    }

    #[test]
    fn test_map_result_fails() {
        let items = vec![1, 2, 3];
        let result = map_result(items, |x| {
            if x == 2 {
                Err(Error::validation_error("fail on 2"))
            } else {
                Ok(x)
            }
        });
        assert!(result.is_err());
    }

    // ── filter_result additional cases ───────────────────────────────────────

    #[test]
    fn test_filter_result_empty() {
        let result = filter_result::<i32, _>(vec![], |x| Ok(*x > 0));
        assert_eq!(result.expect("ok"), Vec::<i32>::new());
    }

    #[test]
    fn test_filter_result_all_pass() {
        let items = vec![1, 2, 3];
        let result = filter_result(items, |x| Ok(*x < 10));
        assert_eq!(result.expect("ok"), vec![1, 2, 3]);
    }

    #[test]
    fn test_filter_result_none_pass() {
        let items = vec![1, 2, 3];
        let result = filter_result(items, |x| Ok(*x > 10));
        assert_eq!(result.expect("ok"), Vec::<i32>::new());
    }

    #[test]
    fn test_filter_result_fails() {
        let items = vec![1, 2, 3];
        let result = filter_result(items, |x| {
            if *x == 2 {
                Err(Error::validation_error("fail on 2"))
            } else {
                Ok(true)
            }
        });
        assert!(result.is_err());
    }

    // ── Type alias smoke tests ───────────────────────────────────────────────

    #[test]
    fn test_type_aliases_are_usable() {
        // FallibleTransform<T, U> = fn(T) -> Result<U>
        let transform: FallibleTransform<i32, String> = |x| Ok(x.to_string());
        assert_eq!(transform(42).expect("ok"), "42");

        // Validator<T> = fn(&T) -> Result<()>
        let validator: Validator<i32> = |x| {
            if *x > 0 {
                Ok(())
            } else {
                Err(Error::validation_error("must be positive"))
            }
        };
        assert!(validator(&1).is_ok());
        assert!(validator(&0).is_err());
    }
}
