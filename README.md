# Cluster Computing

## Project Introduction
ClusterComputing is a distributed computing platform developed in Rust, focusing on image recognition. This project aims to build an efficient distributed system for processing and analyzing large-scale image data. By parallel processing tasks in a multi-node environment, ClusterComputing significantly enhances the efficiency and speed of image processing.

## Main Features
- **Node Communication:** Utilizes TcpStream for stable communication between nodes.
- **Node Monitoring:** Real-time monitoring of each node to ensure stable operation.
- **Task and File Transfer:** Efficient transmission of task information and file transfer.
- **Image Recognition:** Integrates Yolov8 for efficient image recognition.

## Technology Stack
- **Rust:** Fully leverages the high performance and safety features of Rust.
- **Actix-web:** Offers a user-friendly web management interface.
- **Tokio:** Uses Tokio's asynchronous runtime for performance optimization.

## Installation and Configuration
1. Clone the repository: `git clone https://github.com/DaLaw2/ClusterComputing`
2. Enter the project directory and run `cargo build` to compile the project.

## How to Use
- Run the compiled application directly to start the system.
- Install Yolov8 and the client on the nodes to process image recognition tasks.

## Error Handling and Security
- Proactively handles all errors and enhances system stability and security through logging.
