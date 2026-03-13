/**
 * CLI Plugin Registry HTTP API
 *
 * Registry for native CLI plugins with platform-specific builds.
 */

import "@typespec/http";

using TypeSpec.Http;

namespace CliPluginRegistry;

// -- Models --

model CliPluginEntry {
  id: string;
  name: string;
  description: string;
  latestVersion: string;
  downloads: uint64;
  author: string;
  tags: string[];
}

model PlatformBuild {
  platform: string;
  downloadUrl: string;
  sizeBytes: uint64;
  checksum: string;
  signature?: string;
}

model CliPluginInfo {
  id: string;
  version: string;
  platforms: PlatformBuild[];
  publishedAt: uint64;
  changelog?: string;
  previewUrl?: string;
  previewImages: string[];
}

model CliRegistryIndex {
  version: uint32;
  updatedAt: uint64;
  plugins: CliPluginEntry[];
}

model CliSearchResults {
  plugins: CliPluginEntry[];
}

model SearchQuery {
  @query q: string;
}

model PublishResponse {
  status: string;
  id: string;
  version: string;
}

model PublishParams {
  @query name: string;
  @query description?: string;
  @query author?: string;
}

// -- Interfaces --

interface IndexService {
  @get
  @route("/v1/index.json")
  getIndex(): {
    @statusCode statusCode: 200;
    @body body: CliRegistryIndex;
  };
}

interface SearchService {
  @get
  @route("/v1/search")
  search(...SearchQuery): {
    @statusCode statusCode: 200;
    @body body: CliSearchResults;
  };
}

@route("/v1")
interface PluginService {
  @get
  @route("/{id}/latest.json")
  getLatest(@path id: string): {
    @statusCode statusCode: 200;
    @body body: CliPluginInfo;
  };

  @get
  @route("/{id}/{version}.json")
  getVersion(@path id: string, @path version: string): {
    @statusCode statusCode: 200;
    @body body: CliPluginInfo;
  };

  @get
  @route("/{id}/{version}/{platform}.tar.gz")
  download(@path id: string, @path version: string, @path platform: string): {
    @statusCode statusCode: 200;
    @body body: bytes;
  };

  @get
  @route("/{id}/{version}/preview.html")
  previewPage(@path id: string, @path version: string): {
    @statusCode statusCode: 200;
    @body body: bytes;
  };

  @get
  @route("/{id}/{version}/preview/{index}.webp")
  previewImage(@path id: string, @path version: string, @path index: string): {
    @statusCode statusCode: 200;
    @body body: bytes;
  };
}

@route("/v1/publish")
interface PublishService {
  @post
  @route("/{id}/{version}/{platform}")
  publish(@path id: string, @path version: string, @path platform: string, ...PublishParams, @body body: bytes): {
    @statusCode statusCode: 201;
    @body body: PublishResponse;
  };
}
