# Quality Assurance Strategy (QA Strategy) - Wormhole

**Project:** Wormhole (Distributed File System)
**Last Updated:** January 2026
**Status:** Applied

---

## 1. Vision and Objectives

Since Wormhole is a **file system**, data loss or corruption is unacceptable. Unlike a web application where a bug might only require a page refresh, an error in Wormhole could destroy user files.

Our QA strategy rests on three fundamental pillars:

1. **Data Integrity First:** No operation must compromise stored data.
2. **Reproducibility:** The test environment must be stable and predictable.
3. **Pyramidal Validation:** From fast unit tests to industry-standard compliance tests (XFSTests).

---

## 2. The Quality Environment

### 2.1. Nix & Flakes (Highly Recommended)

While using **Nix** is not a strict requirement for contributing, it is **strongly recommended** by the core team.

* **Why use NixOS:** NixOS and Flakes offer a superior development experience by ensuring everyone uses exactly the same tool versions (`cargo`, `fuse3`, `openssl`). This almost entirely eliminates environment-related bugs ("It works on my machine").
* **For Nix Users:** A `flake.lock` file is provided to instantly configure a dev environment identical to production.

### 2.2. Static Analysis and Formatting

Code quality is verified before compilation to maintain a healthy codebase.

* **Formatting:** `cargo fmt` is used to standardize code style.
* **Linting:** `clippy` is used to detect Rust anti-patterns and potential security issues.
* **Memory Safety:** The choice of **Rust** natively guarantees the absence of *Buffer Overflows* or *Data Races* in Safe code.

---

## 3. The Test Pyramid

We apply a 4-layer testing strategy, ranging from granular to global.

### Level 1: Unit Tests

* **Objective:** Validate internal module logic (e.g., file slicing, directory tree management).
* **Tool:** Native Rust framework `cargo test`.
* **Location:** In the source code (`src/`) alongside the tested functions.
* **Execution:** On every commit via CI.

### Level 2: Functional Tests (Integration Tests)

* **Objective:** Validate communication between Wormhole nodes (Pods) in a simulated network.
* **Method:** We developed an internal test harness, the `EnvironmentManager`, capable of spawning multiple isolated Wormhole daemons and simulating network scenarios.
* **Covered Scenarios:**
* Initial synchronization (`test_sync.rs`).
* File and folder transfer (`test_transfer.rs`).
* Resilience to service interruptions (`test_sending_files_on_stop.rs`).

### Level 3: System Tests & Compliance

* **Objective:** Prove that Wormhole behaves like a "real" Linux file system (POSIX compliance).
* **Tool:** **XFSTests** (QA System). This is the standard test suite used by Linux kernel developers (ext4, btrfs, xfs).
* **Implementation:**
* Containerization via Docker to isolate execution.
* Mounting Wormhole via FUSE and running generic test suites.

### Level 4: Acceptance Tests (Beta Testing)

* **Objective:** Validate User Experience (UX) and complex use cases (Installation, Windows).
* **Reference:** `ATP.md` document.
* **Process:** Manual execution of scenarios defined in the ATP (Acceptance Test Plan) before each major Release.

---

## 4. Development Process (Workflow)

We use the **Gitflow** model to secure the production branch.

1. **Branches:**

* `main`: Stable version, production-ready. Cannot be pushed to directly.
* `dev`: Integration version.
* `feat/*`: Development branches.

1. **Code Review:**

* No code can be merged into `dev` without a **Pull Request (PR)**.
* Every PR requires approval from at least one other developer (Pair Review).
* The review verifies: logic, readability, and associated test coverage.

1. **Quality Gates (CI/CD):**

* CI (GitHub Actions) is blocking.
* **Gate 1:** Compilation & Formatting (Linux & Windows).
* **Gate 2:** Unit & Functional Tests (Must all pass).
* **Gate 3:** XFSTests (System regression tests).
