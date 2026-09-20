import { invoke } from "@tauri-apps/api/core";
import type {
  TranscriptionAuthType,
  TranscriptionEndpointInput,
  TranscriptionEndpointTestResult,
  TranscriptionEndpointView,
} from "@/bindings";

export type {
  TranscriptionAuthType,
  TranscriptionEndpointInput,
  TranscriptionEndpointTestResult,
  TranscriptionEndpointView,
};

export const emptyEndpointInput = (): TranscriptionEndpointInput => ({
  id: null,
  name: "",
  base_url: "http://localhost:8000/v1",
  transcription_path: "/audio/transcriptions",
  model: "",
  auth_type: "bearer",
  auth_header_name: "",
  send_language: true,
  extra_params_json: "",
  timeout_secs: 60,
});

export const listTranscriptionEndpoints = () =>
  invoke<TranscriptionEndpointView[]>("list_transcription_endpoints");

export const saveTranscriptionEndpoint = (
  input: TranscriptionEndpointInput,
  secret?: string | null,
) =>
  invoke<TranscriptionEndpointView>("save_transcription_endpoint", {
    input,
    secret: secret || null,
  });

export const deleteTranscriptionEndpoint = (id: string) =>
  invoke<void>("delete_transcription_endpoint", { id });

export const duplicateTranscriptionEndpoint = (id: string) =>
  invoke<TranscriptionEndpointView>("duplicate_transcription_endpoint", { id });

export const setActiveTranscriptionEndpoint = (id: string) =>
  invoke<void>("set_active_transcription_endpoint", { id });

export const setTranscriptionEndpointSecret = (id: string, secret: string) =>
  invoke<void>("set_transcription_endpoint_secret", { id, secret });

export const clearTranscriptionEndpointSecret = (id: string) =>
  invoke<void>("clear_transcription_endpoint_secret", { id });

export const testTranscriptionEndpoint = (id: string) =>
  invoke<TranscriptionEndpointTestResult>("test_transcription_endpoint", {
    id,
  });
