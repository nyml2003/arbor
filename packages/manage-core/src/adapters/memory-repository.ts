import type { TaskRepository, TaskStoreDocument } from "../domain/types.js";
import { emptyTaskDocument } from "../domain/types.js";

export function createMemoryTaskRepository(
  initial: TaskStoreDocument = emptyTaskDocument(),
): TaskRepository {
  let document = cloneDocument(initial);

  return {
    async load() {
      return cloneDocument(document);
    },
    async save(nextDocument) {
      document = cloneDocument(nextDocument);
    },
  };
}

function cloneDocument(document: TaskStoreDocument): TaskStoreDocument {
  return JSON.parse(JSON.stringify(document)) as TaskStoreDocument;
}
