pub fn conjunction<T: ToString>(list: &[T]) -> String {
    if list.is_empty() {
        "".to_string()
    } else if list.len() == 1 {
        list[0].to_string()
    } else {
        let mut result = String::new();
        for (i, item) in list.iter().enumerate() {
            if i == list.len() - 1 {
                result.push_str(" and ");
            } else if i > 0 {
                result.push_str(", ");
            }

            result.push_str(&item.to_string());
        }

        result
    }
}
