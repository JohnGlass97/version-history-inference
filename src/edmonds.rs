use chu_liu_edmonds::chu_liu_edmonds;
use ndarray::{arr2, ArrayView2};

pub fn find_msa(scores: ArrayView2<f32>, root_vertex: usize) -> Vec<Option<usize>> {
    let new_scores = &scores * -1.0;
    return chu_liu_edmonds(new_scores.view(), root_vertex);
}

pub fn msa_to_string(result: &Vec<Option<usize>>) -> String {
    result
        .iter()
        .map(|opt| match opt {
            Some(x) => x.to_string(),
            None => String::from("_"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_msa_simple_mst() {
        let distance_graph = arr2(&[
            [0., 99., 100., 100.],
            [99., 0., 1., 1.],
            [100., 1., 0., 5.],
            [100., 1., 5., 0.],
        ]);

        let res = find_msa(distance_graph.view(), 0);

        assert_eq!(msa_to_string(&res), "_, 0, 1, 1");
    }

    #[test]
    fn test_find_msa_simple_msa() {
        let distance_graph = arr2(&[
            [0., 99., 100., 99.],
            [99., 0., 10., 10.],
            [100., 1., 0., 5.],
            [99., 2., 5., 0.],
        ]);

        let res = find_msa(distance_graph.view(), 0);

        assert_eq!(msa_to_string(&res), "_, 2, 3, 0");
    }
}
