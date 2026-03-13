/**
 * Web Plugin Registry HTTP API
 *
 * Registry for web plugins (JS/CSS bundles with optional preview assets).
 */

import "@typespec/http";

using TypeSpec.Http;

namespace WebPluginRegistry;

model WebPluginEntry {
  id: string;
  name: string;
  description: string;
  latestVersion: string;
  downloads: uint64;
  author: string;
  tags: string[];
}

model WebPluginInfo {
  id: string;
  version: string;
  jsUrl: string;
  cssUrl?: string;
  sizeBytes: uint64;
  publishedAt: uint64;
  changelog?: string;
  previewUrl?: string;
  previewImages: string[];
}

model WebRegistryIndex {
  version: uint32;
  updatedAt: uint64;
  plugins: WebPluginEntry[];
}

model WebSearchResults {
  plugins: WebPluginEntry[];
}

model SearchQuery {
  @query q: string;
}

model PublishResponse {
  status: string;
  id: string;
  version: string;
}

interface IndexService {
  @get
  @route("/v1/index.json")
  getIndex(): {
    @statusCode statusCode: 200;
    @body body: WebRegistryIndex;
  };
}

interface SearchService {
  @get
  @route("/v1/search")
  search(...SearchQuery): {
    @statusCode statusCode: 200;
    @body body: WebSearchResults;
  };
}

@route("/v1")
interface PluginService {
  @get
  @route("/{id}/latest.json")
  getLatest(@path id: string): {
    @statusCode statusCode: 200;
    @body body: WebPluginInfo;
  };

  @get
  @route("/{id}/{version}.json")
  getVersion(@path id: string, @path version: string): {
    @statusCode statusCode: 200;
    @body body: WebPluginInfo;
  };

  @get
  @route("/{id}/{version}/main.js")
  downloadJs(@path id: string, @path version: string): {
    @statusCode statusCode: 200;
    @body body: bytes;
  };

  @get
  @route("/{id}/{version}/main.css")
  downloadCss(@path id: string, @path version: string): {
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
  @route("/{id}/{version}")
  publish(@path id: string, @path version: string, @body body: bytes): {
    @statusCode statusCode: 201;
    @body body: PublishResponse;
  };
}
