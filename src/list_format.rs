pub fn conjunction<T: ToString>(list: &[T]) -> String {
    match list.len() {
        0 => "".to_string(),
        1 => list[0].to_string(),
        _ => list
            .iter()
            .enumerate()
            .flat_map(|(i, item)| {
                vec![
                    match i {
                        _ if i == list.len() - 1 => " and ".to_string(),
                        _ if i > 0 => ", ".to_string(),
                        _ => "".to_string(),
                    },
                    item.to_string(),
                ]
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}
