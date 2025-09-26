## 🦀 Engineering Plan: `rust-testscript` Crate

### **1. Project Vision & Core Principles**

The goal is to create `rust-testscript`, a crate for testing command-line tools using filesystem-based script files, mirroring the functionality and developer experience of Go's `rogpeppe/go-internal/testscript`.

  * **Idiomatic Rust:** The library must feel natural to Rust developers. This means using `Result` for error handling, leveraging iterators, employing the builder pattern for configuration, and using traits for extensibility.
  * **Modularity:** Components like the parser, environment manager, and command executor should be distinct and testable in isolation.
  * **Extensibility:** Users must be able to easily define their own custom commands and conditions, just like in the Go version.
  * **Minimal Dependencies:** We will rely on well-vetted, popular crates where necessary (e.g., for temp files, command execution) but avoid unnecessary bloat.

### **2. High-Level Architecture**

The library will consist of several key components that work together:

1.  **Test Runner:** The main entry point for the user. It discovers test script files and orchestrates their execution.
2.  **Script Parser:** Responsible for parsing the `.txtar` format into a structured representation of commands and files.
3.  **Execution Environment:** Manages the temporary, isolated directory for each test script run, including file setup and environment variables.
4.  **Command Engine:** A dispatcher that interprets and executes parsed commands (e.g., `exec`, `cmp`, `stdout`) against the execution environment.
5.  **Configuration (`RunParams`):** A builder struct that allows users to customize the test run, such as by adding custom commands or setup logic.

-----

### **3. Phase 1: The Core Engine (MVP)**

This phase focuses on building the non-extensible core functionality. The goal is to successfully parse and run a basic test script with built-in commands.

#### **Task 3.1: Project Scaffolding**

  * Initialize a new Rust library crate: `cargo new --lib rust-testscript`.
  * Set up the initial `Cargo.toml` with metadata (authors, license, description).
  * Add initial dependencies:
      * `anyhow` for simple, flexible error handling.
      * `thiserror` for creating custom, structured error types.
      * `tempfile` for creating isolated temporary directories for test runs.
      * `walkdir` for discovering test script files.

#### **Task 3.2: Implement the `txtar` Parser**

  * Create a new module: `mod parser;`.
  * **Data Structures:** Define structs to represent the parsed script.
    ```rust
    // Represents a single file block in the archive
    pub struct TxtarFile {
        pub name: String,
        pub contents: Vec<u8>,
    }

    // Represents the parsed script and its associated files
    pub struct Script {
        pub commands: Vec<Command>,
        pub files: Vec<TxtarFile>,
    }

    // Represents a single command line in the script
    pub struct Command {
        pub name: String,
        pub args: Vec<String>,
        pub line_num: usize,
    }
    ```
  * **Parsing Logic:** Implement a function `parser::parse(content: &str) -> Result<Script>`. This function will perform a line-by-line parse.
    1.  It should handle the file preamble (`-- filename --`).
    2.  It must correctly extract file contents until the next preamble or the end of the file.
    3.  It must parse command lines, splitting them into a command name and arguments, while correctly handling quoted arguments.
    4.  Ignore lines starting with `#` (comments).
    5.  Return a structured `Script` object.

#### **Task 3.3: The Execution Environment**

  * Create a new module: `mod run;`.
  * **`TestEnvironment` Struct:** This struct will manage the state for a single script execution.
    ```rust
    use tempfile::TempDir;
    use std::path::PathBuf;
    use std::collections::HashMap;

    pub struct TestEnvironment {
        // The root temporary directory for the test run.
        pub work_dir: PathBuf,
        // The underlying TempDir that cleans up on drop.
        _temp_dir: TempDir,
        // Environment variables for this specific run.
        pub env_vars: HashMap<String, String>,
    }
    ```
  * **Implementation:**
    1.  `TestEnvironment::new()`: Creates a new `TempDir` using the `tempfile` crate. Sets `work_dir` to its path.
    2.  `setup_files(&self, files: &[TxtarFile])`: A method that takes the files from the parsed `Script` and writes them into `work_dir`. It must handle creating subdirectories as needed.

#### **Task 3.4: The Main Test Runner & Command Engine**

  * **`run_test` function:** This will be the main public function in the crate.
    ```rust
    // In lib.rs
    pub fn run_test(script_path: &Path) -> Result<()> {
        // ... implementation ...
    }
    ```
  * **Implementation Steps:**
    1.  Read the script file content from `script_path`.
    2.  Use the `parser` to parse the content into a `Script` object.
    3.  Create a new `TestEnvironment`.
    4.  Call `env.setup_files()` to populate the working directory.
    5.  **Command Loop:** Iterate through the `script.commands`.
    6.  **Command Dispatch:** Use a `match` statement on `command.name` to dispatch to built-in command handlers.
          * **`exec`:**
              * Use `std::process::Command` to execute the command.
              * Set the `current_dir` to the `env.work_dir`.
              * Inject the `env.env_vars`.
              * Capture `stdout`, `stderr`, and the exit code.
              * Store this output in a state variable for subsequent checks (e.g., for `stdout` and `stderr` commands).
          * **`cmp`:**
              * Read the contents of the two specified files within `work_dir`.
              * Compare them. Return an error if they don't match.
          * **`stdout` / `stderr`:**
              * Compare the captured output from the *last* `exec` command with the provided argument (or a file's content).
              * Support basic regular expressions via the `regex` crate.
          * **`cd`:**
              * Update a `current_subdir` path within the `TestEnvironment` struct. All subsequent `exec` commands will use this updated path relative to `work_dir`.

-----

### **4. Phase 2: Extensibility and Usability**

This phase makes the library configurable and ergonomic for end-users, introducing the builder pattern and mechanisms for custom commands.

#### **Task 4.1: The `RunParams` Builder**

  * Create a `RunParams` struct to hold all configuration.
    ```rust
    // In run.rs or a new params.rs module

    // Type alias for a command function
    pub type CommandFn = fn(&mut TestEnvironment, args: &[String]) -> Result<()>;

    pub struct RunParams {
        // Custom commands provided by the user.
        pub commands: HashMap<String, CommandFn>,
        // Setup function to run before the script executes.
        pub setup: Option<Box<dyn FnOnce(&TestEnvironment) -> Result<()>>>,
        // ... other params to be added later ...
    }

    // Implement the builder pattern for RunParams
    impl RunParams {
        pub fn new() -> Self { /* ... */ }
        pub fn command(mut self, name: &str, func: CommandFn) -> Self { /* ... */ }
        pub fn setup(mut self, func: impl FnOnce(&TestEnvironment) -> Result<()> + 'static) -> Self { /* ... */ }
        // ... etc. ...
    }
    ```

#### **Task 4.2: Integrate `RunParams` into the Runner**

  * Refactor the `run_test` function to accept `RunParams`. The main test discovery logic will now live in a separate function.
    ```rust
    // In lib.rs
    use std::path::Path;

    // The user calls this from their tests/ directory.
    pub fn run(params: &mut RunParams, test_data_glob: &str) {
        // Find all files matching the glob (e.g., "testdata/*.txt").
        // For each file, call run_script.
        // Panic on the first failure to match `go test` behavior.
    }

    // Internal function that runs a single script.
    fn run_script(path: &Path, params: &RunParams) -> Result<()> {
        // ... existing logic from run_test ...
    }
    ```
  * **Refactor Command Dispatch:** Modify the command loop in `run_script`.
    1.  First, check if the command name exists in `params.commands`. If so, execute the user-provided function.
    2.  If not, fall back to the `match` statement for built-in commands.

#### **Task 4.3: Implement the `setup` Hook**

  * In `run_script`, after creating the `TestEnvironment` but before executing the command loop, check if `params.setup` is `Some`.
  * If it is, execute the setup closure, passing it a reference to the `TestEnvironment`. This allows the user to perform actions like compiling a binary into the `work_dir`.
      * **Example Usage:** A user would write:
        ```rust
        // in tests/integration_test.rs
        #[test]
        fn run_all_scripts() {
            let mut params = rust_testscript::RunParams::new();
            params = params.setup(|env| {
                // Compile the main binary into the test's temp directory
                let status = std::process::Command::new("cargo")
                    .args(["build", "--bin", "my-cli"])
                    .status()?;
                assert!(status.success());
                // Copy binary to work_dir
                std::fs::copy("target/debug/my-cli", env.work_dir.join("my-cli"))?;
                Ok(())
            });
            rust_testscript::run(&mut params, "testdata/*.txt");
        }
        ```

-----

### **5. Phase 3: Advanced Features & Polish**

This phase adds features for more complex scenarios and improves the overall quality of the crate.

#### **Task 5.1: Implement Conditions (`[condition]`)**

  * **Parser Update:** Modify the `parser` to recognize conditional prefixes on commands (e.g., `[windows] exec ...`). Store the condition in the `Command` struct.
  * **`RunParams` Update:** Add a `conditions` map.
    ```rust
    // In RunParams
    pub conditions: HashMap<String, bool>,
    ```
  * **Runner Update:** In the command loop, before executing a command, check if it has a condition.
      * If it does, look up the condition in `params.conditions`.
      * If the condition is present and `true`, execute the command.
      * If the condition is present and `false`, skip the command.
      * If the condition is *not* present, fail the test with an "unknown condition" error.
  * **Default Conditions:** Pre-populate the `conditions` map with useful defaults like `windows`, `linux`, `mac`, `unix`.

#### **Task 5.2: Implement Background Commands (`&`)**

  * **Parser Update:** Recognize the `&` suffix on `exec` commands.
  * **Runner Update:**
      * When an `exec` command has the `&` suffix, spawn the process using `std::process::Command::spawn()`.
      * Store the `Child` process handle in a `background_pids` map in the `TestEnvironment`. Give it a name (e.g., the first argument).
      * Implement a new built-in command: `wait <name>`. This command will find the named background process in the map and call `wait()` on it, capturing its output.

#### **Task 5.3: Documentation and Examples**

  * Write comprehensive doc comments (`///`) for all public functions and structs, explaining their purpose and usage.
  * Create an `examples/` directory in the crate with a simple CLI tool and a corresponding `tests/` directory that uses `rust-testscript` to test it. This will serve as a reference implementation.
  * Write a detailed `README.md` that explains the philosophy, provides a quick-start guide, and documents all built-in commands and features.
