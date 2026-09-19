import { invoke } from "@tauri-apps/api/core";

export type TranscriptionAuthType = "none" | "bearer" | "custom_header";

export interface TranscriptionEndpointView {
  id: string;
  name: string;
  base_url: string;
  transcription_path: string;
  model: string;
  auth_type: TranscriptionAuthType;
  auth_header_name: string | null;
  send_language: boolean;
  extra_params_json: string;
  timeout_secs: number;
  is_preset: boolean;
  is_active: boolean;
  has_secret: boolean;
}

export interface TranscriptionEndpointInput {
  id?: string | null;
  name: string;
  base_url: string;
  transcription_path: string;
  model: string;
  auth_type: TranscriptionAuthType;
  auth_header_name?: string | null;
  send_language: boolean;
  extra_params_json: string;
  timeout_secs: number;
}

export interface TranscriptionEndpointTestResult {
  ok: boolean;
  message: string;
}

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

export const saveTranscriptionEndpoint = (input: TranscriptionEndpointInput) =>
  invoke<TranscriptionEndpointView>("save_transcription_endpoint", { input });

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
