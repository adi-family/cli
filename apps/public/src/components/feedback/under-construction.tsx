"use client";

import { createComponent } from "@lit/react";
import { AdiUnderConstruction as AdiUnderConstructionElement } from "@adi-family/sdk-ui-components/feedback";
import React from "react";

export const UnderConstruction = createComponent({
  tagName: "adi-under-construction",
  elementClass: AdiUnderConstructionElement,
  react: React,
});
