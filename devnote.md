# DevNote
## Architecture Changes
This is the planned architecture change for version 2.0:

- Abandon the "out-of-the-box" design concept and add more configuration options
- Implement persistent storage using SQLite
- Expand recognition capabilities: In addition to existing static media recognition (images, videos), add support for media streams
- Expected outcome: Establish a highly available recognition system for media streams

## Refactor
- Rewrite front end page
- Deprecate actix-web-actors and use actix-ws
- Transfer the logging system implemented in the project to tracing
- Use event system to improve existing architecture

## Feature:
- Upgrade Tcp connection to tls connection.
- Consider handing over the video encoding/decoding work to a dedicated Agent.

## Bug fixes:
