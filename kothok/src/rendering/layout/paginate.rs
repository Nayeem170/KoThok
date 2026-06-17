pub fn paginate_with_heights_ext(
    heights: &[i32],
    content_h: i32,
    heading_indices: &[usize],
) -> Vec<(usize, usize)> {
    let heading_set: std::collections::HashSet<usize> = heading_indices.iter().copied().collect();
    let mut pages = Vec::new();
    let n = heights.len();
    let mut i = 0usize;
    while i < n {
        let start = i;
        let mut h = 0i32;
        while i < n {
            let rh = heights[i];
            if h + rh > content_h && i > start {
                break;
            }
            h += rh;
            i += 1;
        }
        let page_end = i;
        if page_end < n {
            let last = page_end - 1;
            if heading_set.contains(&last) && i < n && !heading_set.contains(&i) {
                i = last;
            }
        }
        pages.push((start, i));
    }
    if pages.is_empty() {
        pages.push((0, 0));
    }
    pages
}
