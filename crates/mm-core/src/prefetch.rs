/// Determine which pages to prefetch based on current position.
pub fn prefetch_pages(
    current_page: usize,
    total_pages: usize,
    forward: usize,
    backward: usize,
) -> Vec<usize> {
    let mut pages = Vec::new();

    // Forward pages
    for i in 1..=forward {
        let p = current_page + i;
        if p < total_pages {
            pages.push(p);
        }
    }

    // Backward pages
    for i in 1..=backward {
        if current_page >= i {
            pages.push(current_page - i);
        }
    }

    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefetch_forward() {
        let pages = prefetch_pages(5, 100, 2, 0);
        assert_eq!(pages, vec![6, 7]);
    }

    #[test]
    fn test_prefetch_backward() {
        let pages = prefetch_pages(5, 100, 0, 2);
        assert_eq!(pages, vec![4, 3]);
    }

    #[test]
    fn test_prefetch_both() {
        let pages = prefetch_pages(5, 100, 2, 2);
        assert_eq!(pages, vec![6, 7, 4, 3]);
    }

    #[test]
    fn test_prefetch_at_start() {
        let pages = prefetch_pages(0, 100, 2, 2);
        assert_eq!(pages, vec![1, 2]); // No backward pages
    }

    #[test]
    fn test_prefetch_at_end() {
        let pages = prefetch_pages(99, 100, 2, 2);
        assert_eq!(pages, vec![98, 97]); // No forward pages
    }

    #[test]
    fn test_prefetch_near_end() {
        let pages = prefetch_pages(98, 100, 2, 2);
        assert_eq!(pages, vec![99, 97, 96]); // Only 1 forward page
    }

    #[test]
    fn test_prefetch_small_archive() {
        let pages = prefetch_pages(0, 2, 2, 2);
        assert_eq!(pages, vec![1]); // Only 1 forward, no backward
    }

    #[test]
    fn test_prefetch_single_page() {
        let pages = prefetch_pages(0, 1, 2, 2);
        assert!(pages.is_empty());
    }

    #[test]
    fn test_prefetch_zero_config() {
        let pages = prefetch_pages(5, 100, 0, 0);
        assert!(pages.is_empty());
    }
}
