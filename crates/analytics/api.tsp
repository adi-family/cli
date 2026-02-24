/**
 * Analytics API
 *
 * REST API for querying analytics metrics and dashboards.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace AnalyticsApi;

// -- Models --

model DateRangeParams {
  @query startDate?: string;
  @query endDate?: string;
}

model OverviewStats {
  totalUsers: int64;
  activeUsersToday: int64;
  activeUsersWeek: int64;
  activeUsersMonth: int64;
  totalTasks: int64;
  tasksToday: int64;
  taskSuccessRate: float64;
  totalCocoons: int64;
  activeCocoons: int64;
  totalIntegrations: int64;
}

model DailyActiveUsers {
  day: utcDateTime;
  activeUsers: int64;
  totalEvents: int64;
}

model TaskStats {
  day: utcDateTime;
  created: int64;
  started: int64;
  completed: int64;
  failed: int64;
  cancelled: int64;
  avgDurationMs?: float64;
  p95DurationMs?: float64;
  successRate: float64;
}

model TaskStatsOverview {
  totalCreated: int64;
  totalCompleted: int64;
  totalFailed: int64;
  totalCancelled: int64;
  successRate: float64;
  avgDurationMs: float64;
}

model EndpointLatency {
  hour: utcDateTime;
  service: string;
  endpoint: string;
  method: string;
  requestCount: int64;
  avgDurationMs: float64;
  p50DurationMs: float64;
  p95DurationMs: float64;
  p99DurationMs: float64;
  error4xxCount: int64;
  error5xxCount: int64;
}

// -- Interface --

interface AnalyticsService {
  @get
  @route("/overview")
  getOverview(): {
    @statusCode statusCode: 200;
    @body body: OverviewStats;
  };

  @get
  @route("/users/daily")
  getDailyActiveUsers(...DateRangeParams): {
    @statusCode statusCode: 200;
    @body body: DailyActiveUsers[];
  };

  @get
  @route("/users/weekly")
  getWeeklyActiveUsers(...DateRangeParams): {
    @statusCode statusCode: 200;
    @body body: DailyActiveUsers[];
  };

  @get
  @route("/tasks/daily")
  getTaskStatsDaily(...DateRangeParams): {
    @statusCode statusCode: 200;
    @body body: TaskStats[];
  };

  @get
  @route("/tasks/overview")
  getTaskStatsOverview(...DateRangeParams): {
    @statusCode statusCode: 200;
    @body body: TaskStatsOverview;
  };

  @get
  @route("/api/latency")
  getEndpointLatency(...DateRangeParams): {
    @statusCode statusCode: 200;
    @body body: EndpointLatency[];
  };

  @get
  @route("/api/slowest")
  getSlowestEndpoints(): {
    @statusCode statusCode: 200;
    @body body: EndpointLatency[];
  };
}
