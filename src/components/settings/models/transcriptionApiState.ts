export const API_TRANSCRIPTION_MODEL_ID = "api-transcription";

/**
 * An endpoint is active only while API transcription is the selected engine.
 * The preferred endpoint remains stored when a local model is selected so the
 * user can return to it without reconfiguration.
 */
export const isEndpointEffectivelyActive = (
  endpointSelected: boolean,
  apiModelActive: boolean,
): boolean => endpointSelected && apiModelActive;
