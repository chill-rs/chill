# Chill-rs Change Log

## v0.2.0 (unreleased)

### Breaking changes

* Replace each owning and non-owning path-related type pair (e.g.,
  `DatabaseName` and `DatabaseNameRef`) with a single owning type (e.g.,
  `DatabaseName`) ([issue #33](issue_33)). This increases the number of
  memory allocations in some use cases but vastly simplifies the API.
* Convert action-constructing `Client` methods to be infallible ([issue
  #34](issue_34)). This simplifies the API.
* Remove deprecated `Document::id` method ([issue #42](issue_42)).

## v0.1.2 (unreleased)

* Deprecate the `Document::id` method, use `Document::path` instead.
* New support for `limit` query parameter when executing a view.

## v0.1.1 (2016-04-16)

The v0.1.1 release extends Chill's coverage of the CouchDB API.

* New support for executing views (`action::ExecuteView`).
* New support for creating, reading, updating, and deleting attachments
  as part of reading and updating documents. However, Chill still has no
  support for accessing attachments individually.

## v0.1.0 (2016-03-26)

This is the first release. It has minimal support for creating, reading,
updating, and deleting documents.

[issue_33]: https://github.com/chill-rs/chill/issues/33
[issue_34]: https://github.com/chill-rs/chill/issues/34
[issue_42]: https://github.com/chill-rs/chill/issues/42
