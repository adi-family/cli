/**
 * Tasks HTTP API
 *
 * Task management with dependencies, graph analysis, and symbol linking.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace Tasks;

// -- Enums --

enum TaskStatus {
  todo,
  inProgress: "in_progress",
  done,
  blocked,
  cancelled,
}

// -- Models --

model Task {
  id: int64;
  title: string;
  description?: string;
  status: TaskStatus;
  symbolId?: int64;
  projectPath?: string;
  createdAt: int64;
  updatedAt: int64;
}

model TaskWithDependencies {
  task: Task;
  dependsOn: Task[];
  dependents: Task[];
}

model TasksStatus {
  totalTasks: uint64;
  todoCount: uint64;
  inProgressCount: uint64;
  doneCount: uint64;
  blockedCount: uint64;
  cancelledCount: uint64;
  totalDependencies: uint64;
  hasCycles: boolean;
}

model CreateTaskInput {
  title: string;
  description?: string;
  dependsOn?: int64[];
  symbolId?: int64;
}

model UpdateTaskInput {
  title?: string;
  description?: string;
  status?: string;
  symbolId?: int64;
}

model UpdateStatusInput {
  status: string;
}

model AddDependencyInput {
  dependsOn: int64;
}

model ListQuery {
  @query status?: string;
}

model SearchQuery {
  @query q: string;
  @query limit?: int32;
}

model GraphNode {
  task: Task;
  dependencies: int64[];
}

model IdResponse {
  id: int64;
}

model DeletedResponse {
  deleted: int64;
}

model DependencyResponse {
  from: int64;
  to: int64;
}

model RemovedResponse {
  removed: boolean;
}

model CyclesResponse {
  cycles: int64[][];
}

model LinkResponse {
  taskId: int64;
  symbolId: int64;
}

model UnlinkResponse {
  taskId: int64;
  unlinked: boolean;
}

// -- Interfaces --

@route("/tasks")
interface TaskService {
  @get
  list(...ListQuery): {
    @statusCode statusCode: 200;
    @body body: Task[];
  };

  @post
  create(@body body: CreateTaskInput): {
    @statusCode statusCode: 201;
    @body body: IdResponse;
  };

  @get
  @route("/{id}")
  get(@path id: int64): {
    @statusCode statusCode: 200;
    @body body: TaskWithDependencies;
  };

  @put
  @route("/{id}")
  update(@path id: int64, @body body: UpdateTaskInput): {
    @statusCode statusCode: 200;
    @body body: Task;
  };

  @delete
  @route("/{id}")
  delete(@path id: int64): {
    @statusCode statusCode: 200;
    @body body: DeletedResponse;
  };

  @put
  @route("/{id}/status")
  updateStatus(@path id: int64, @body body: UpdateStatusInput): {
    @statusCode statusCode: 200;
    @body body: Task;
  };

  @get
  @route("/{id}/dependencies")
  getDependencies(@path id: int64): {
    @statusCode statusCode: 200;
    @body body: Task[];
  };

  @post
  @route("/{id}/dependencies")
  addDependency(@path id: int64, @body body: AddDependencyInput): {
    @statusCode statusCode: 201;
    @body body: DependencyResponse;
  };

  @delete
  @route("/{id}/dependencies/{depId}")
  removeDependency(@path id: int64, @path depId: int64): {
    @statusCode statusCode: 200;
    @body body: RemovedResponse;
  };

  @get
  @route("/{id}/dependents")
  getDependents(@path id: int64): {
    @statusCode statusCode: 200;
    @body body: Task[];
  };

  @get
  @route("/search")
  search(...SearchQuery): {
    @statusCode statusCode: 200;
    @body body: Task[];
  };

  @get
  @route("/ready")
  getReady(): {
    @statusCode statusCode: 200;
    @body body: Task[];
  };

  @get
  @route("/blocked")
  getBlocked(): {
    @statusCode statusCode: 200;
    @body body: Task[];
  };

  @put
  @route("/{id}/link/{symbolId}")
  linkToSymbol(@path id: int64, @path symbolId: int64): {
    @statusCode statusCode: 200;
    @body body: LinkResponse;
  };

  @delete
  @route("/{id}/link")
  unlinkSymbol(@path id: int64): {
    @statusCode statusCode: 200;
    @body body: UnlinkResponse;
  };
}

@route("/graph")
interface GraphService {
  @get
  getGraph(): {
    @statusCode statusCode: 200;
    @body body: GraphNode[];
  };

  @get
  @route("/cycles")
  detectCycles(): {
    @statusCode statusCode: 200;
    @body body: CyclesResponse;
  };
}

@route("/status")
interface StatusService {
  @get
  getStatus(): {
    @statusCode statusCode: 200;
    @body body: TasksStatus;
  };
}
