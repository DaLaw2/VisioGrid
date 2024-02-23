# VisioGrid

## Project Introduction
VisioGrid is a distributed computing platform developed with Rust, focusing on image recognition. This project aims to establish an efficient distributed system for processing large-scale image data. By parallel processing tasks in a multi-agent environment, VisioGrid improves the efficiency and speed of image processing.

## Main Features
- **Agent Communication**: Implements stable communication between agents using TcpStream.
- **Agent Monitoring**: Real-time monitoring of each agent to ensure stable operation.
- **Task and File Transfer**: Efficient transmission of task information and files.
- **Image Recognition**: Supports a variety of image recognition models.

## Technology Stack
- **Rust**: Fully leverages the high performance and safety features of Rust.
- **Actix Web**: Provides a user-friendly web management interface.
- **tokio**: Uses Tokio's asynchronous runtime for performance optimization.
- **tch-rs**: Performs deep learning model inference using Torch's Rust bindings.
- **GStreamer**: Used for handling media streams and media content.

## Installation and Configuration
- Clone the repository: `git clone https://github.com/DaLaw2/VisioGrid`
- From source code compilation:
    - Compile the management node (Management) requires installing GStreamer: `cargo build --release --package Management`
    - Compile the agent (Agent) requires installing LibTorch: `cargo build --release --package Agent`
- Running with Docker:
    - The Docker containers for the management node (Management) and agent (Agent) include all necessary dependencies, no manual installation required.

## How to Use
- Compile from source code or use Docker containers to run the management node and agents according to your needs.
- When running with Docker containers, the build and run commands are as follows:
    - Build the management node container: `docker build -t management-image ./Docker/Management`
    - Run the management node container: `docker run management-image`
    - Build the agent container: `docker build -t agent-image ./Docker/Agent`
    - Run the agent container: `docker run agent-image`
- Containers run without the need for any commands, automatically configuring based on initial settings.

## Error Handling and Security
- Proactively handles all errors and enhances system stability and security through logging.
