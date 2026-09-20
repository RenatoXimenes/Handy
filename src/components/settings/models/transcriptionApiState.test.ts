import assert from "node:assert/strict";
import { isEndpointEffectivelyActive } from "./transcriptionApiState";

assert.equal(isEndpointEffectivelyActive(true, true), true);
assert.equal(isEndpointEffectivelyActive(true, false), false);
assert.equal(isEndpointEffectivelyActive(false, true), false);

console.log("transcription API state: all assertions passed");
