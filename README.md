# Dependency suggest

This rust program will download the latest version having the same major version. Then it will check if the latest version contains any vulnerabilities.

## Why Rust?

I just want to practice Rust.

## Prerequisite

`Dependency-check` CLI is required to perform vulnerability check.

You can download from the [official site](https://jeremylong.github.io/DependencyCheck/dependency-check-cli/index.html).

## Build

```
cargo build
```

## Execute

```
DEPENDENCY_CHECK_SCRIPT=<Path of dependency-check.sh> ./dependency-suggest <Group ID> <Artifact ID> <Version>
```

Example:

```
DEPENDENCY_CHECK_SCRIPT=~/Downloads/dependency-check/bin/dependency-check.sh ./dependency-suggest org.apache.logging.log4j log4j-core 2.17.0
```
