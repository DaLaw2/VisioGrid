# ClusterComputing

## Project Introduction

ClusterComputing is a distributed computing platform for image recognition developed in Rust. This project aims to build an efficient distributed system for processing and analyzing large-scale image data. By parallel processing tasks in a multi-node environment, ClusterComputing significantly enhances the efficiency and speed of image processing.

## Main Features

- **Node Communication**: Establishes a stable communication system between nodes, supporting effective data and task information transmission.
- **Task Information Transmission**: Implements accurate delivery of task-related information across distributed nodes.
- **File Transfer**: Provides an efficient file transfer mechanism for transmitting image files between nodes.
- **Node Monitoring**: Features real-time monitoring to ensure the stable operation of each node.
- **Image Recognition Result Retrieval**: Collects and integrates image recognition results from the distributed processing workflow.

## Technology Stack

- **Rust**: Develops the entire project utilizing the high performance and safety features of Rust.
- **Actix-web**: Offers a user-friendly web management interface through the Actix-web framework, facilitating task creation and system monitoring.
- **Tokio**: Extensively uses the Tokio asynchronous runtime to optimize IO operations, enhancing overall performance.
