use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn parse_maze_file(path: &str) -> Vec<Vec<bool>> {
    let file = File::open(path).expect("Failed to open maze file");
    let reader = BufReader::new(file);

    reader
        .lines()
        .map(|line| {
            line.expect("Failed to read line")
                .chars()
                .map(|c| c == '#')
                .collect()
        })
        .collect()
}
