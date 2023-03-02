use aureum::formats::tree;
use aureum::formats::tree::Tree::{self, Leaf, Node};
use aureum::test_id::TestId;
use aureum::toml_config::{
    TestCaseValidationError, TomlConfigData, TomlConfigError, ValidTomlConfig,
};
use relative_path::RelativePathBuf;
use std::collections::BTreeSet;

pub fn print_files_found(source_files: &[RelativePathBuf]) {
    let heading = format!("🔍 Found {} config files", source_files.len());
    let tree = Node(
        heading,
        source_files
            .iter()
            .map(|x| Leaf(vec![x.to_string()]))
            .collect(),
    );

    print_tree(tree);
}

pub fn print_config_details(source_file: RelativePathBuf, config: &ValidTomlConfig) {
    let errors = test_cases_errors(config);
    let tree = Node(config_heading(source_file), errors);

    print_tree(tree);
}

pub fn print_toml_config_error(source_file: RelativePathBuf, error: TomlConfigError) {
    let msg = match error {
        TomlConfigError::FailedToReadFile(_) => "Failed to read file",
        TomlConfigError::FailedToParseTomlConfig(_) => "Failed to parse config file",
    };
    let tree = Node(
        config_heading(source_file),
        vec![Leaf(vec![msg.to_owned()])],
    );

    print_tree(tree);
}

fn print_tree(tree: Tree) {
    let content = tree::draw_tree(&tree).unwrap_or_else(|_| String::from("Failed to draw tree\n"));

    eprint!("{}", content); // Already contains newline
    eprintln!()
}

fn config_heading(source_file: RelativePathBuf) -> String {
    format!("📋 {}", source_file)
}

pub fn any_issues_in_toml_config(config: &ValidTomlConfig) -> bool {
    !config.validation_errors.is_empty()
}

pub fn test_cases_errors(test_cases: &ValidTomlConfig) -> Vec<Tree> {
    let mut categories = vec![];

    let requirements = requirements_map(&test_cases.requirements);
    if !requirements.is_empty() {
        categories.push(Node(String::from("Requirements"), requirements));
    }

    let validation_errors = validation_errors_map(&test_cases.validation_errors);
    if !validation_errors.is_empty() {
        categories.push(Node(String::from("Validation errors"), validation_errors));
    }

    categories
}

fn requirements_map(requirements: &TomlConfigData) -> Vec<Tree> {
    let mut categories = vec![];

    let any_files_missing = requirements.any_missing_file_requirements();
    let files = requirements.file_requirements();
    if any_files_missing && !files.is_empty() {
        categories.push(Node(
            String::from("Files"),
            files
                .into_iter()
                .map(|(x, y)| Leaf(vec![format!("{} {}", show_presence(y), x)]))
                .collect(),
        ));
    }

    let any_env_missing = requirements.any_missing_env_requirements();
    let env = requirements.env_requirements();
    if any_env_missing && !env.is_empty() {
        categories.push(Node(
            String::from("Environment"),
            env.into_iter()
                .map(|(x, y)| Leaf(vec![format!("{} {}", show_presence(y), x)]))
                .collect(),
        ));
    }

    categories
}

fn validation_errors_map(
    validation_errors: &Vec<(TestId, BTreeSet<TestCaseValidationError>)>,
) -> Vec<Tree> {
    if validation_errors.len() == 1 {
        let (maybe_root, errs) = &validation_errors[0];
        if maybe_root.is_root() {
            return errs
                .iter()
                .map(|err| Leaf(vec![show_validation_error(err)]))
                .collect::<Vec<_>>();
        }
    }

    let mut test_cases = vec![];

    for (test_id, errs) in validation_errors {
        test_cases.push(Node(
            test_id.to_string(),
            errs.iter()
                .map(|err| Leaf(vec![show_validation_error(err)]))
                .collect(),
        ));
    }

    test_cases
}

fn show_validation_error(validation_error: &TestCaseValidationError) -> String {
    let msg = match validation_error {
        TestCaseValidationError::MissingExternalFile(file_path) => {
            format!("Missing external file '{}'", file_path)
        }
        TestCaseValidationError::MissingEnvVar(var_name) => {
            format!("Missing environment variable '{}'", var_name)
        }
        TestCaseValidationError::FailedToParseString => String::from("Failed to parse string"),
        TestCaseValidationError::ProgramRequired => String::from("The field 'program' is required"),
        TestCaseValidationError::ProgramNotFound(program) => {
            format!("The program '{}' was not found", program)
        }
        TestCaseValidationError::ExpectationRequired => {
            String::from("At least one expectation is required")
        }
    };

    format!("❌ {}", msg)
}

fn show_presence(value: bool) -> String {
    String::from(if value { "✔️" } else { "❌" })
}
