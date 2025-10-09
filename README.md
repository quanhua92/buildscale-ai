# buildscale-ai

## Directory Structure

The project is organized into the following top-level directories:

-   **/backend**: Contains the Rust API. This includes all the source code, dependencies (`Cargo.toml`), and examples for the server-side logic.
-   **/frontend**: Contains the TanStack Start (React) single-page application. This is where all the UI components, routing, and client-side logic reside.
-   **/docs**: A place for all project documentation. This includes setup guides, API specifications, and architectural diagrams in Markdown (`.md`) format.
-   **/scripts**: Holds various automation and utility scripts (`.sh`) to streamline common development tasks like building, testing, or deploying the applications.

## Getting Started

### Prerequisites

-   Rust and Cargo
-   Node.js and pnpm

### Installation

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/quanhua92/buildscale-ai
    cd buildscale-ai
    ```

2.  **Install frontend dependencies:**
    ```bash
    cd frontend
    pnpm install
    ```

## Usage

### Running the Backend

From the `/backend` directory, you can run the API server:

```bash
cd backend
cargo run
```

### Running the Frontend

From the `/frontend` directory, you can start the development server:

```bash
cd frontend
pnpm run dev
```
