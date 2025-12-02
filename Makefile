# Makefile to streamline One Engine workflows

.PHONY: dev seed canvas ide gov ide-build ide-start

# Start the Rust engine locally (port 7777)
dev:
	./run_dev.sh

# Seed the engine with curated endpoints/events (requires engine running)
seed:
	./scripts/seed_graph_demo.sh

# Open the conversation canvas (serves conversation.md at http://localhost:8000)
canvas:
	./scripts/open_canvas.sh

# Next.js IDE: install deps and run dev server at http://localhost:3000
ide:
	cd apps/ide && npm install && npm run dev

# Build and start the Next.js IDE in production mode
ide-build:
	cd apps/ide && npm install && npm run build

ide-start:
	cd apps/ide && npm start

# Run governance checks via conversational API (engine must be reachable)
gov:
	./scripts/governance_check.sh
