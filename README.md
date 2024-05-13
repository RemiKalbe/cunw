# cunw

cunw (codebase unwrap) is a command-line interface (CLI) tool that generates a structured representation of a codebase, making it easy to provide context to a large language model (LLM). It recursively traverses a directory, collects file content, and generates a single Markdown file that represents the structure and content of the codebase.

> [!WARNING]
> Please note that cunw is currently in a very early and experimental stage. It has not been extensively tested and may be prone to crashes or unexpected behavior. However, rest assured that any crashes will be limited to the tool itself and will not cause any harm to your system or files.

## 🌟 Features

- Recursively traverses a directory and collects file content
- Generates a Markdown file representing the codebase structure and content
- Supports excluding files based on glob patterns
- Respects `.gitignore` files by default (can be disabled)
- Allows specifying the maximum depth of directory traversal
- Supports following symbolic links (disabled by default)

## 📦 Installation

### Precompiled Binaries

You can easily install cunw through cargo:

```bash
cargo install cunw
```

Or download the precompiled binaries from the [releases page](https://github.com/RemiKalbe/cunw/releases).

### From Source

To install cunw, ensure you have Rust and Cargo installed on your system. Then, clone the repository and build the project:

```bash
git clone https://github.com/RemiKalbe/cunw.git
cd cunw
cargo build --release
```

The compiled binary will be available at `target/release/cunw`.

## 🚀 Usage

```bash
cunw [OPTIONS]
```

### Options

- `-p, --path <PATH>`: The path to the directory containing the codebase.
- `-o, --output <FILE>`: The path of the output file. Default: `output.txt`
- `-e, --exclude <PATTERN>`: Exclude files or directories matching the specified glob pattern.
- `--do-not-consider-ignore-files`: Do not consider the ignore files (`.gitignore`, `.hgignore`, `.ignore`, `.git/info/exclude`, and `core.excludesFile` in `.git/config`). Default: `false`
  Note, for now, only `.gitignore` is supported.
- `--dangerously-allow-dot-git-traversal`: Include `.git` directory in the search. Default: `false`
- `-d, --max-depth <DEPTH>`: Maximum depth to walk into the directory tree.
- `-f, --follow-symbolic-links`: Follow symbolic links. Default: `false`
- `-v, --verbose`: Set the verbosity level. Can be used multiple times to increase verbosity.

### Example

To generate a Markdown representation of a codebase located at `path/to/codebase`, excluding files matching `*.txt` and save the output to `codebase.md`:

```bash
cunw path/to/codebase -o codebase.md -e "*.txt"
```

## 📝 Output Format

The generated Markdown file will have the following structure:

```markdown
<directory_structure>
.
└─ .
├─ ./src
│ ├─ main.rs
│ └─ lib.rs
├─ .gitignore
├─ Cargo.lock
└─ Cargo.toml
</directory_structure>

<file path="Cargo.toml">
[package]
name = "cunw"
version = "0.1.0"
edition = "2021"

[dependencies]

<!-- ... -->
</file>

<file path="src/main.rs">
fn main() {
    println!("Hello, world!");
}
</file>

<!-- ... -->
```

The `<directory_structure>` section represents the directory tree of the codebase, and each `<file>` section contains the content of a specific file.

## 🤝 Contributing

Contributions are welcome! If you find any issues or have suggestions for improvements, please open an issue or submit a pull request on the GitHub repository.

## 📄 License

This project is licensed under the [MIT License](LICENSE).
