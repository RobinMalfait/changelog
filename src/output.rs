use colored::*;

/// Small wrapper to have a nice output that is indented and contains a CHANGELOG header. Also
/// playing with some eprintln so that piping it to another process or redirecting it to a file
/// doesn't contain all the extra stuff.
pub fn output(str: String) {
    eprintln!();

    if str.contains('\n') {
        eprintln!("  {}\n", " CHANGELOG ".black().on_bright_blue().bold());

        output_indented(str);
    } else {
        eprint!("  {} ", " CHANGELOG ".black().on_bright_blue().bold());
        println!("{}", str);
    }

    eprintln!()
}

pub fn output_indented(str: String) {
    let str = str.trim();
    let lines = str.lines();
    let total_lines = lines.clone().count();

    for (row_idx, line) in lines.enumerate() {
        eprint!("  ");
        if row_idx < total_lines {
            println!("{}", line);
        } else {
            print!("{}", line);
        }
    }
}
