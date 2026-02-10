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
/// - Each sum of two distinct unit vectors along each pair of dimensions.
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
        // Sum of two distinct dimensions:
        for i in 0..N {
            for j in (i + 1)..N {
                compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                    a.0[i] + a.0[j] < b.0[i] + b.0[j]
                });
                compare_and_replace_tag_opt(&mut dop_tags, &mut idx, tag, |a, b| {
                    a.0[i] + a.0[j] > b.0[i] + b.0[j]
                });
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
/// - Each sum of two distinct unit vectors along each pair of dimensions.
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
        // Sum of two distinct dimensions:
        for i in 0..N {
            for j in (i + 1)..N {
                compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| {
                    a[i] + a[j] < b[i] + b[j]
                });
                compare_and_replace_coord_opt(&mut dop_coords, &mut idx, coord, |a, b| {
                    a[i] + a[j] > b[i] + b[j]
                });
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
