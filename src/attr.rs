///! Types and tests for the gitattributes functionality of git and libgit2.

/// The return value from git_attr_get.
///
/// This can be either `TRUE`, `FALSE`, `UNSPECIFIED` (if the attribute
/// was not set at all), or `VALUE`, if the attribute was set to an
/// actual string.
#[derive(Debug, PartialEq, Eq)]
pub enum AttributeType {
    /// The attribute was not set at all.
    Unspecified,

    /// The attribute is true for this file path.
    True,

    /// The attribute is false for this file path.
    False,

    /// The attribute has a string value associated with it.
    Value(String)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempdir::TempDir;
    use {Repository};
    use {CheckAttributeFlags, AttributeType};

    #[test]
    fn attribute_smoke() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path).unwrap();
        let result = repo.get_attr(CheckAttributeFlags::empty(), "file.txt", "testattr").unwrap();

        assert_eq!(result, AttributeType::Unspecified);
    }

    #[test]
    fn attribute_is_true() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path).unwrap();

        let filepath = repo.workdir().unwrap().join(".gitattributes");
        fs::write(filepath, "*.txt testattr").unwrap();

        let result = repo.get_attr(CheckAttributeFlags::empty(), "file.txt", "testattr").unwrap();
        assert_eq!(result, AttributeType::True);
    }

    #[test]
    fn attribute_is_false() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path).unwrap();

        let filepath = repo.workdir().unwrap().join(".gitattributes");
        fs::write(filepath, "*.txt -testattr").unwrap();

        let result = repo.get_attr(CheckAttributeFlags::empty(), "file.txt", "testattr").unwrap();
        assert_eq!(result, AttributeType::False);
    }

    #[test]
    fn attribute_is_value() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path).unwrap();

        let filepath = repo.workdir().unwrap().join(".gitattributes");
        fs::write(filepath, "*.txt testattr=hello").unwrap();

        let result = repo.get_attr(CheckAttributeFlags::empty(), "file.txt", "testattr").unwrap();
        assert_eq!(result, AttributeType::Value("hello".to_string()));
    }

    #[test]
    fn complex_attributes() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path).unwrap();

        let filepath = repo.workdir().unwrap().join(".gitattributes");
        fs::write(filepath,
                  "*.* txt
                            *.sh shell run=bash
                            *.bin -txt binary"
        ).unwrap();

        let text = repo.get_attr(CheckAttributeFlags::empty(), "file.txt", "txt").unwrap();
        assert_eq!(text, AttributeType::True);

        let shell = repo.get_attr(CheckAttributeFlags::empty(), "script.sh", "shell").unwrap();
        assert_eq!(shell, AttributeType::True);
        let run = repo.get_attr(CheckAttributeFlags::empty(), "script.sh", "run").unwrap();
        assert_eq!(run, AttributeType::Value("bash".to_string()));
        let text_shell = repo.get_attr(CheckAttributeFlags::empty(), "script.sh", "txt").unwrap();
        assert_eq!(text_shell, AttributeType::True);

        let bin_txt = repo.get_attr(CheckAttributeFlags::empty(), "data.bin", "txt").unwrap();
        assert_eq!(bin_txt, AttributeType::False);
        let bin_binary = repo.get_attr(CheckAttributeFlags::empty(), "data.bin", "binary").unwrap();
        assert_eq!(bin_binary, AttributeType::True);
    }
}
