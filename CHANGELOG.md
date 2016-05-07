# Chill-rs Change Log

## v0.2.0 (unreleased)

No changes yet!

## v0.1.2 (2016-05-07)

The v0.1.2 release has a few small changes.

* New support for the `limit` query parameter when executing a view.
* Deprecate the `Document::id` method. Applications should use
  `Document::path` instead.
* Define `IntoUrl` not to be based on Hyper's trait of the same name.

## v0.1.1 (2016-04-16)

The v0.1.1 release extends Chill's coverage of the CouchDB API.

* New support for executing views (`action::ExecuteView`).
* New support for creating, reading, updating, and deleting attachments
  as part of reading and updating documents. However, Chill still has no
  support for accessing attachments individually.

## v0.1.0 (2016-03-26)

This is the first release. It has minimal support for creating, reading,
updating, and deleting documents.
