// Remove dist/ so deleted sources don't linger in the served bundle.
import { rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

rmSync(join(dirname(fileURLToPath(import.meta.url)), "..", "dist"), { recursive: true, force: true });
