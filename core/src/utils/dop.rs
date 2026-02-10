fn compare_and_replace_tag_opt<const N: usize, Cmp>(
    tags: &mut [Option<([usize; N], usize)>],
    idx: &mut usize,
    tag: ([usize; N], usize),
    cmp: Cmp,
) where
    Cmp: Fn(&([usize; N], usize), &([usize; N], usize)) -> bool,
{
    match &mut tags[*idx] {
        Some(existing_tag) => {
            if cmp(&tag, existing_tag) {
                *existing_tag = tag;
            }
        }
        None => {
            tags[*idx] = Some(tag);
        }
    }
    *idx += 1;
}

/// Computes the Discrete Oriented Polytope (DOP) tags for a set of given tags.
///
/// Specifically, for each coordinate in the input tags, return only the tags that have mimimal and
/// maximal projections against the following vectors:
/// - Each unit vector along each dimension,
/// - Each sum or difference of two distinct unit vectors along each pair of dimensions.
///
/// The algorithm returns at most 2N + 2N(N-1) = 2N^2 tags for N-dimensional coordinates.
pub fn get_dop_tags<const N: usize>(tags: &[([usize; N], usize)]) -> Vec<([usize; N], usize)> {
    // Since the tag is (N+1)-dimensional (including the spawn dim)...
    let mut dop_tags: Vec<Option<([usize; N], usize)>> = vec![None; 2 * (N + 1) * (N + 1)];
    for &tag in tags {
        let mut idx = 0;

        // Compare against the current dop tags and update as needed.
        // Unit across non-spawn dimensions:
        for i in 0..N {
            compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| a.0[i] < b.0[i]);
            compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| a.0[i] > b.0[i]);
        }
        // Unit of spawn dimension:
        compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| a.1 < b.1);
        compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| a.1 > b.1);
        // Sum or difference of two distinct dimensions:
        for i in 0..N {
            for j in (i + 1)..N {
                compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                    a.0[i] + a.0[j] < b.0[i] + b.0[j]
                });
                compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                    a.0[i] + a.0[j] > b.0[i] + b.0[j]
                });
                // Comparison against <1, -1, 0, ...> replaced with additive form to avoid
                // usize subtraction
                compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                    a.0[i] + b.0[j] < b.0[i] + a.0[j]
                });
                compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                    a.0[i] + b.0[j] > b.0[i] + a.0[j]
                });
            }
            compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                a.0[i] + a.1 < b.0[i] + b.1
            });
            compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                a.0[i] + a.1 > b.0[i] + b.1
            });
            compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                a.0[i] + b.1 < b.0[i] + a.1
            });
            compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                a.0[i] + b.1 > b.0[i] + a.1
            });
        }
    }
    dop_tags
        .into_iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

fn compare_and_replace_coord_opt<const N: usize, Cmp>(
    coords: &mut [Option<[usize; N]>],
    idx: &mut usize,
    coord: [usize; N],
    cmp: Cmp,
) where
    Cmp: Fn(&[usize; N], &[usize; N]) -> bool,
{
    match &mut coords[*idx] {
        Some(existing_coord) => {
            if cmp(&coord, existing_coord) {
                *existing_coord = coord;
            }
        }
        None => {
            coords[*idx] = Some(coord);
        }
    }
    *idx += 1;
}

/// Computes the Discrete Oriented Polytope (DOP) coordinates for a set of given coordinates.
///
/// Specifically, for each coordinate in the input coordinates, return only the coordinates that
/// have mimimal and maximal projections against the following vectors:
/// - Each unit vector along each dimension,
/// - Each sum or difference of two distinct unit vectors along each pair of dimensions.
///
/// The algorithm returns at most 2N + 2N(N-1) = 2N^2 tags for N-dimensional coordinates.
pub fn get_dop_coords<const N: usize>(coords: &[[usize; N]]) -> Vec<[usize; N]> {
    // Since the tag is N-dimensional...
    let mut dop_coords: Vec<Option<[usize; N]>> = vec![None; 2 * N * N];
    for &coord in coords {
        let mut idx = 0;

        // Compare against the current dop coords and update as needed.
        // Unit across dimensions:
        for i in 0..N {
            compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| a[i] < b[i]);
            compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| a[i] > b[i]);
        }
        // Sum or difference of two distinct dimensions:
        for i in 0..N {
            for j in (i + 1)..N {
                compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| {
                    a[i] + a[j] < b[i] + b[j]
                });
                compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| {
                    a[i] + a[j] > b[i] + b[j]
                });
                // Comparison against <1, -1, 0, ...> replaced with additive form to avoid
                // usize subtraction
                compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| {
                    a[i] + b[j] < b[i] + a[j]
                });
                compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| {
                    a[i] + b[j] > b[i] + a[j]
                });
            }
        }
    }
    dop_coords
        .into_iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn set_of<T: std::hash::Hash + Eq>(v: Vec<T>) -> HashSet<T> {
        v.into_iter().collect()
    }

    #[test]
    fn dop_coords_empty() {
        let coords: Vec<[usize; 2]> = vec![];
        let result = get_dop_coords::<2>(&coords);
        let result_set = set_of(result);

        let expected: HashSet<[usize; 2]> = HashSet::new();
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_coords_single_point() {
        let coords = vec![[3, 7]];
        let result = get_dop_coords::<2>(&coords);
        let result_set = set_of(result);

        let expected = set_of(vec![[3, 7]]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_coords_two_points_axis_extrema() {
        let coords = vec![[0, 10], [10, 0]];
        let result = get_dop_coords::<2>(&coords);
        let result_set = set_of(result);

        // Both are extrema along some axis or diagonal
        let expected = set_of(vec![[0, 10], [10, 0]]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_coords_square_2d() {
        // Four corners of a square
        let coords = vec![[0, 0], [0, 10], [10, 0], [10, 10]];
        let result = get_dop_coords::<2>(&coords);
        let result_set = set_of(result);

        // All corners should remain as extrema
        let expected = set_of(vec![[0, 0], [0, 10], [10, 0], [10, 10]]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_coords_interior_removed() {
        let coords = vec![[0, 0], [0, 10], [10, 0], [10, 10], [5, 5]];
        let result = get_dop_coords::<2>(&coords);
        let result_set = set_of(result);

        // Interior point should not be part of the DOP
        let expected = set_of(vec![[0, 0], [0, 10], [10, 0], [10, 10]]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_coords_duplicates() {
        let coords = vec![[1, 2], [1, 2], [1, 2]];
        let result = get_dop_coords::<2>(&coords);
        let result_set = set_of(result);

        let expected = set_of(vec![[1, 2]]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_tags_empty() {
        let tags: Vec<([usize; 2], usize)> = vec![];
        let result = get_dop_tags::<2>(&tags);
        let result_set = set_of(result);
        let expected: HashSet<([usize; 2], usize)> = HashSet::new();
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_tags_single_point() {
        let tags = vec![([3, 7], 5)];
        let result = get_dop_tags::<2>(&tags);
        let result_set = set_of(result);

        let expected = set_of(vec![([3, 7], 5)]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_tags_extrema() {
        let tags = vec![([0, 0], 0), ([10, 0], 0), ([0, 10], 0), ([10, 10], 0)];
        let result = get_dop_tags::<2>(&tags);
        let result_set = set_of(result);

        let expected = set_of(vec![([0, 0], 0), ([10, 0], 0), ([0, 10], 0), ([10, 10], 0)]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_tags_interior_removed() {
        let tags = vec![
            ([0, 0], 0),
            ([10, 0], 0),
            ([0, 10], 0),
            ([10, 10], 0),
            ([5, 5], 0), // interior
        ];
        let result = get_dop_tags::<2>(&tags);
        let result_set = set_of(result);

        let expected = set_of(vec![([0, 0], 0), ([10, 0], 0), ([0, 10], 0), ([10, 10], 0)]);
        assert_eq!(result_set, expected);
    }

    #[test]
    fn dop_tags_spawn_dimension_extrema() {
        let tags = vec![([5, 5], 0), ([5, 5], 10), ([5, 5], 20)];
        let result = get_dop_tags::<2>(&tags);
        let result_set = set_of(result);

        // Only min and max along spawn dimension should remain
        let expected = set_of(vec![([5, 5], 0), ([5, 5], 20)]);
        assert_eq!(result_set, expected);
    }
}
