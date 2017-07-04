# Chill

Chill is a client-side CouchDB library for the Rust programming
language, available on [crates.io][chill_crates_io]. It targets Rust
Stable.

Chill's three chief design goals are **convenience**, **safety**, and
**efficiency**.

You can read more about its design and development here:

* [Rethinking CouchDB in Rust][cv_rethinking_couchdb] (2016-02-23)
* [Announcing Chill v0.1.1][cv_announcing_chill_v0_1_1] (2016-04-18)

## Roadmap

Chill's most recent release is **v0.3.0**, available as of **2016-10-01**.

* [v0.3.0 change log][v0_3_0_change_log]
* [v0.3.0 documentation][v0_3_0_documentation]
* [v0.3.0 issues][v0_3_0_issues]
* [v0.3.0 crates.io page][v0_3_0_crates_io]

The next release will be **v0.4.0** and will entail:

* A much-needed refresh of dependencies, Serde v1.x perhaps being the
  most important, and,
* Possibly an overhaul of the API to support asynchronous I/O via the
  Tokio model.

## License

Chill is licensed under either of:

* **Apache License, Version 2.0**, ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0), or,
* **MIT License** ([LICENSE-MIT](LICENSE-MIT) or
  http://opensource.org/licenses/MIT).

## Feedback

Do you find this crate useful? Not useful? [Please send
feedback][feedback_email]!

[couchdb_github]: https://github.com/couchdb-rs/couchdb
[chill_crates_io]: https://crates.io/crates/chill
[cv_announcing_chill_v0_1_1]: https://cmbrandenburg.github.io/post/2016-04-18-chill_v0.1.1/
[cv_rethinking_couchdb]: https://cmbrandenburg.github.io/post/2016-02-23-rethinking_couchdb_in_rust/
[feedback_email]: mailto:c.m.brandenburg@gmail.com
[master_change_log]: https://github.com/chill-rs/chill/blob/master/CHANGELOG.md
[v0_3_0_change_log]: https://github.com/chill-rs/chill/blob/v0.3.0/CHANGELOG.md
[v0_3_0_crates_io]: https://crates.io/crates/chill/0.3.0
[v0_3_0_documentation]: https://chill-rs.github.io/chill/doc/v0.3.0/chill/
[v0_3_0_issues]: https://github.com/chill-rs/chill/issues?q=milestone%3Av0.3.0
