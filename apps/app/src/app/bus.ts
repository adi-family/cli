import { createEventBus } from "@adi-family/sdk-plugin";
import { getGlobal, setGlobal } from "./global";

setGlobal({
  bus: createEventBus(),
})

export function getBus() { return getGlobal().bus }
