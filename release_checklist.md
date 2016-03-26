# CouchDB-rs Release Checklist

1. Resolve any `FIXME` comments in the code.

1. Ensure the `chill` crate builds and runs using the latest
   dependencies.

        $ cargo update &&
          cargo test &&
          cargo test --release

  If any errors occur then fix them.

1. Ensure packaging succeeds.

        $ cargo package

  If any errors occur then fix them.

1. Create a temporary Git branch for the release.

        $ git checkout -b release_prep

1. Update project files.

    1. Edit `Cargo.toml` to declare the correct version for this
       crate.

        1. E.g., remove the `+master` suffix.

        1. Ensure the documentation link is correct.

    1. Edit `CHANGELOG.md` to add the date for the new release and
       remove the “unreleased” adjective. Ensure the change log is
       up-to-date, clear, and well formatted.

    1. Edit `README.md` to update all references to the latest release
       and next release.

    1. Ensure there are no untracked files in the working directory.

    1. Commit changes.

            $ git commit

1. Build and publish Rust documentation for the new version.

    1. Build.

            $ cargo clean &&
              cargo doc --no-deps &&
              ver=$(grep '^version' Cargo.toml | head -n1 | sed -e 's/.*\([[:digit:]]\+\.[[:digit:]]\+\.[[:digit:]]\+\).*/\1/') &&
              git checkout gh-pages &&
              cp -a target/doc doc/v$ver &&
              git add doc/v$ver

    1. Review `doc/v$ver/chill/index.html`.

    1. Publish.

            $ git commit -a -m "Add documentation for v$ver" &&
              git push origin &&
              git checkout release_prep

1. Merge updates into master.

        $ git checkout master &&
          git merge release_prep &&
          git branch -d release_prep

1. Publish the crate.

        $ cargo publish

1. Create Git tag.

        $ git tag -a v$ver -m "v$ver release" &&
          git push --tags

1. Prep for new work.

    1. Edit `Cargo.toml` to increment the version, adding the `+master`
       suffix.

    1. Edit `CHANGELOG.md` to add the new “unreleased” section for the
       next version.

    1. Commit changes

            $ git commit

    1. [Close the issue milestone for the new release](
       https://github.com/chill-rs/chill/milestones).
