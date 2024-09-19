# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.3] - 2024-09-19

### Fixed

- Fixed the normalization of the `exclude` option when the path is `.` or `./`.
- Fixed the edge case where the root directory is ignored by the `.gitignore` when the path is `.` or `./` and the `gitignore` has `.*` as an ignore pattern.

## [0.2.2] - 2024-09-18

### Added

- Added unit tests for `args`, `codebase`, and `gitignore` modules.
- Added unit tests for the `tree` module.

### Fixed

- Switch from custom `GitIgnore` implementation to the [ignore](https://crates.io/crates/ignore) crate (fix [#1](https://github.com/RemiKalbe/cunw/issues/1)).
- `GIT_RELATED_IGNORE_PATTERNS` constant now correctly ignore `.git` directories.

### Changed

- Created a new `Tree` structure replacing `termtree` which was cumbersome to use (fix [#1](https://github.com/RemiKalbe/cunw/issues/1)).
- Refactored the `Codebase` struct to use the new `Tree` structure (fix [#1](https://github.com/RemiKalbe/cunw/issues/1)).
- Automatically normalize the `exclude` to make it easier to use (fix [#2](https://github.com/RemiKalbe/cunw/issues/2)).
- Switched from `nuclei` to `tokio` for asynchronous file reading (fix [#4](https://github.com/RemiKalbe/cunw/issues/4)).

## [0.2.1] - 2024-05-13

### Fixed

- The `path` property in the generated file is now correctly formatted inside double quotes.

## [0.2.0] - 2024-05-13

### Added

- Introduced a new `CodebaseBuilder` struct to facilitate the construction of the codebase representation.
- Implemented a `GitIgnore` struct to handle parsing and matching of `.gitignore` patterns.
- Introduced a new `FileContent` struct to handle asynchronous file content loading.
- Added crate `nuclei` for asynchronous file reading. (Need to evaluate if it's a good fit)
- Added a `CodebaseItem` enum to represent files and directories in the codebase.
- Implemented a `Codebase` struct to hold the codebase tree and provide formatting methods.
- Introduced an `os` module for platform-specific utilities.

### Changed

- Refactored the codebase traversal logic to use the `CodebaseBuilder` struct.
  The main improvement of this refactoring is the fact that the codebase is only traversed once, simultaneously filtering, collecting, and building the codebase representation.
- Improved error handling and reporting using the `miette` crate.
- Improved logs formatting and verbosity levels.
- Refactored the output file generation to use the `Codebase` struct and its formatting methods.

### Removed

- Removed the `walk` module and integrated its functionality into the `CodebaseBuilder`.
- Removed the `file` module and integrated its functionality into the `codebase` module.
- Removed the `errors` module and replaced it with the `miette` crate for error handling.
- Removed the crate `ptree` and replaced it with `termtree` for tree formatting.
- Removed the crate `ignore-files` and implemented custom logic for `.gitignore` parsing.
- Removed the crate `crossterm` as it was overkill, replaced with `colored`.

### Fixed

- Fixed various issues and improved code quality throughout the codebase.
- Improved overall performance.

## [0.1.0] - 2024-05-11

### Added

- Initial release of cunw
- Recursive directory traversal and file content collection
- Markdown generation representing codebase structure and content
- Support for excluding and including files based on glob patterns
- Respect for `.gitignore` files (can be disabled)
- Option to specify maximum depth of directory traversal
- Option to follow symbolic links (disabled by default)
- Verbose output for debugging purposes

### Fixed

- Minor improvements to error handling and logging

[Unreleased]: https://github.com/RemiKalbe/cunw/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/RemiKalbe/cunw/releases/tag/v0.2.0
[0.1.0]: https://github.com/RemiKalbe/cunw/releases/tag/v0.1.0
