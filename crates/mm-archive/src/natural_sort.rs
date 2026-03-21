/// Natural sort comparison: numeric substrings compared as numbers.
/// "page2" < "page10" (unlike lexicographic where "page10" < "page2").
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let a_num = collect_number(&mut a_chars);
                    let b_num = collect_number(&mut b_chars);
                    match a_num.cmp(&b_num) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    let ac_lower = ac.to_lowercase().next().unwrap_or(ac);
                    let bc_lower = bc.to_lowercase().next().unwrap_or(bc);
                    match ac_lower.cmp(&bc_lower) {
                        std::cmp::Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

fn collect_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u64 {
    let mut num: u64 = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num = num.saturating_mul(10).saturating_add(c as u64 - '0' as u64);
            chars.next();
        } else {
            break;
        }
    }
    num
}

/// Sort a slice of strings in-place using natural ordering.
pub fn natural_sort(items: &mut [String]) {
    items.sort_by(|a, b| natural_cmp(a, b));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sort_basic() {
        let mut items = vec![
            "page10.jpg".into(),
            "page2.jpg".into(),
            "page1.jpg".into(),
            "page20.jpg".into(),
            "page3.jpg".into(),
        ];
        natural_sort(&mut items);
        assert_eq!(
            items,
            vec![
                "page1.jpg",
                "page2.jpg",
                "page3.jpg",
                "page10.jpg",
                "page20.jpg"
            ]
        );
    }

    #[test]
    fn test_natural_sort_mixed() {
        let mut items = vec![
            "img100.png".into(),
            "img2.png".into(),
            "img10.png".into(),
            "img1.png".into(),
        ];
        natural_sort(&mut items);
        assert_eq!(
            items,
            vec!["img1.png", "img2.png", "img10.png", "img100.png"]
        );
    }

    #[test]
    fn test_natural_sort_case_insensitive() {
        let mut items = vec!["B.jpg".into(), "a.jpg".into(), "C.jpg".into()];
        natural_sort(&mut items);
        assert_eq!(items, vec!["a.jpg", "B.jpg", "C.jpg"]);
    }

    #[test]
    fn test_natural_cmp_equal() {
        assert_eq!(natural_cmp("page1", "page1"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_natural_cmp_no_numbers() {
        assert_eq!(natural_cmp("abc", "abd"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_natural_cmp_only_numbers() {
        assert_eq!(natural_cmp("2", "10"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_natural_sort_empty() {
        let mut items: Vec<String> = vec![];
        natural_sort(&mut items);
        assert!(items.is_empty());
    }

    #[test]
    fn test_natural_sort_single() {
        let mut items = vec!["page1.jpg".into()];
        natural_sort(&mut items);
        assert_eq!(items, vec!["page1.jpg"]);
    }
}
