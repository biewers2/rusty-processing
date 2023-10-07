pub mod assertions {
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::{BufRead, BufReader, Read};
    use std::ops::DerefMut;
    use std::path;
    use bytesize::MB;

    use serde_json::Value;

    pub fn assert_identical(expected_path: impl AsRef<path::Path>, actual_path: impl AsRef<path::Path>) {
        let mut exp_read = reader(expected_path);
        let mut act_read = reader(actual_path);

        let mut exp_buf = Box::new([0; MB as usize]);
        let mut act_buf = Box::new([0; MB as usize]);

        while let (Ok(exp_size), Ok(act_size)) = (exp_read.read(exp_buf.deref_mut()), act_read.read(act_buf.deref_mut())) {
            if exp_size == 0 && act_size == 0 {
                break;
            }
            assert_eq!(exp_size, act_size);
            assert_eq!(exp_buf[..exp_size], act_buf[..act_size], "Expected and actual files are different");
        }
    }

    pub fn assert_identical_text(expected_path: impl AsRef<path::Path>, actual_path: impl AsRef<path::Path>) {
        let mut exp_lines = reader(expected_path).lines();
        let mut act_lines = reader(actual_path).lines();

        loop {
            match (exp_lines.next(), act_lines.next())  {
                (Some(exp_line), Some(act_line)) => {
                    let exp_line = exp_line.expect("Failed to read expected line");
                    let act_line = act_line.expect("Failed to read actual line");
                    assert_eq!(exp_line, act_line);
                },
                (None, None) => break,
                _ => panic!("Expected and actual files have different number of lines")
            }
        }
    }

    pub fn assert_identical_metadata(expected_path: impl AsRef<path::Path>, actual_path: impl AsRef<path::Path>) {
        let exp_json: Value = serde_json::from_reader(reader(expected_path)).expect("Failed to deserialize expected metadata");
        let act_json: Value = serde_json::from_reader(reader(actual_path)).expect("Failed to deserialize actual metadata");
        let exp_obj = exp_json.as_object().expect("Expected metadata is not an object");
        let act_obj = act_json.as_object().expect("Actual metadata is not an object");

        let exp_keys: HashSet<String> = exp_obj.keys().map(|k| k.to_owned()).collect();
        let act_keys: HashSet<String> = act_obj.keys().map(|k| k.to_owned()).collect();

        for key in exp_keys.union(&act_keys) {
            let exp_val = exp_obj.get(key).unwrap_or_else(|| panic!("Expected metadata is missing key {:?}", key));
            let act_val = act_obj.get(key).unwrap_or_else(|| panic!("Actual metadata is missing key {:?}", key));
            assert_eq!(exp_val, act_val, "Metadata value for key '{}' is different", key)
        }

    }

    fn reader(path: impl AsRef<path::Path>) -> BufReader<File> {
        let file = File::open(path.as_ref()).unwrap_or_else(|_| panic!("Failed to open file: {:?}", path.as_ref()));
        BufReader::new(file)
    }
}