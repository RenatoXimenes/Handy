import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Copy, Pencil, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import { Input } from "@/components/ui/Input";
import {
  clearTranscriptionEndpointSecret,
  deleteTranscriptionEndpoint,
  duplicateTranscriptionEndpoint,
  emptyEndpointInput,
  listTranscriptionEndpoints,
  saveTranscriptionEndpoint,
  setActiveTranscriptionEndpoint,
  testTranscriptionEndpoint,
  type TranscriptionAuthType,
  type TranscriptionEndpointInput,
  type TranscriptionEndpointView,
} from "@/lib/transcriptionApi";

const authOptions: TranscriptionAuthType[] = [
  "none",
  "bearer",
  "custom_header",
];

export const TranscriptionApiSettings: React.FC = () => {
  const { t } = useTranslation();
  const [endpoints, setEndpoints] = useState<TranscriptionEndpointView[]>([]);
  const [loading, setLoading] = useState(true);
  const [editorOpen, setEditorOpen] = useState(false);
  const [draft, setDraft] =
    useState<TranscriptionEndpointInput>(emptyEndpointInput());
  const [secretDraft, setSecretDraft] = useState("");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [testMessage, setTestMessage] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const next = await listTranscriptionEndpoints();
    setEndpoints(next);
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const next = await listTranscriptionEndpoints();
        if (!cancelled) {
          setEndpoints(next);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const openCreate = () => {
    setDraft(emptyEndpointInput());
    setSecretDraft("");
    setEditorOpen(true);
  };

  const openEdit = (endpoint: TranscriptionEndpointView) => {
    setDraft({
      id: endpoint.id,
      name: endpoint.name,
      base_url: endpoint.base_url,
      transcription_path: endpoint.transcription_path,
      model: endpoint.model,
      auth_type: endpoint.auth_type,
      auth_header_name: endpoint.auth_header_name ?? "",
      send_language: endpoint.send_language,
      extra_params_json: endpoint.extra_params_json,
      timeout_secs: endpoint.timeout_secs,
    });
    setSecretDraft("");
    setEditorOpen(true);
  };

  const handleSave = async () => {
    setError(null);
    setSaving(true);
    try {
      await saveTranscriptionEndpoint(draft, secretDraft.trim() || null);
      setEditorOpen(false);
      setSecretDraft("");
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const runAction = async (id: string, action: () => Promise<void>) => {
    setBusyId(id);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusyId(null);
    }
  };

  const editingPreset = Boolean(
    draft.id &&
      endpoints.find((endpoint) => endpoint.id === draft.id)?.is_preset,
  );

  if (loading) {
    return null;
  }

  return (
    <div className="space-y-3 rounded-xl border-2 border-mid-gray/20 px-4 py-3">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-medium">{t("settings.api.title")}</h2>
          <p className="text-sm text-text/60">
            {t("settings.api.description")}
          </p>
          <p className="mt-1 text-xs text-text/50">
            {t("settings.api.privacyNotice")}
          </p>
        </div>
        <Button variant="primary-soft" size="sm" onClick={openCreate}>
          <Plus className="w-3.5 h-3.5" />
          {t("settings.api.add")}
        </Button>
      </div>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <div className="space-y-2">
        {endpoints.map((endpoint) => (
          <div
            key={endpoint.id}
            className="flex flex-col gap-2 rounded-lg border border-mid-gray/20 px-3 py-2"
          >
            <div className="flex items-center justify-between gap-2">
              <div className="min-w-0">
                <div className="flex items-center gap-2 flex-wrap">
                  <p className="text-sm font-semibold truncate">
                    {endpoint.name}
                  </p>
                  {endpoint.is_active && (
                    <span className="text-xs text-logo-primary">
                      {t("settings.api.active")}
                    </span>
                  )}
                  <span className="text-xs text-text/50">
                    {endpoint.auth_type === "none"
                      ? t("settings.api.keyNotRequired")
                      : endpoint.has_secret
                        ? t("settings.api.keyConfigured")
                        : t("settings.api.keyMissing")}
                  </span>
                </div>
                <p className="text-xs text-text/50 truncate">
                  {endpoint.model || t("settings.api.noModel")} ·{" "}
                  {endpoint.base_url}
                </p>
              </div>
              <div className="flex items-center gap-1 shrink-0">
                {!endpoint.is_active && (
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={busyId === endpoint.id}
                    onClick={() =>
                      runAction(endpoint.id, () =>
                        setActiveTranscriptionEndpoint(endpoint.id),
                      )
                    }
                  >
                    <Check className="w-3.5 h-3.5" />
                    {t("settings.api.activate")}
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={busyId === endpoint.id}
                  onClick={async () => {
                    setBusyId(endpoint.id);
                    try {
                      const result = await testTranscriptionEndpoint(
                        endpoint.id,
                      );
                      setTestMessage((current) => ({
                        ...current,
                        [endpoint.id]: result.message,
                      }));
                    } catch (err) {
                      setError(String(err));
                    } finally {
                      setBusyId(null);
                    }
                  }}
                >
                  {t("settings.api.test")}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={t("settings.api.edit")}
                  title={t("settings.api.edit")}
                  onClick={() => openEdit(endpoint)}
                >
                  <Pencil className="w-3.5 h-3.5" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={t("settings.api.duplicate")}
                  title={t("settings.api.duplicate")}
                  disabled={busyId === endpoint.id}
                  onClick={() =>
                    runAction(endpoint.id, async () => {
                      await duplicateTranscriptionEndpoint(endpoint.id);
                    })
                  }
                >
                  <Copy className="w-3.5 h-3.5" />
                </Button>
                {!endpoint.is_preset && (
                  <Button
                    variant="danger-ghost"
                    size="sm"
                    aria-label={t("common.delete")}
                    title={t("common.delete")}
                    disabled={busyId === endpoint.id}
                    onClick={() =>
                      runAction(endpoint.id, () =>
                        deleteTranscriptionEndpoint(endpoint.id),
                      )
                    }
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </Button>
                )}
              </div>
            </div>
            {testMessage[endpoint.id] && (
              <p className="text-xs text-text/60">{testMessage[endpoint.id]}</p>
            )}
          </div>
        ))}
      </div>

      <Dialog
        open={editorOpen}
        onOpenChange={setEditorOpen}
        title={
          draft.id ? t("settings.api.editTitle") : t("settings.api.createTitle")
        }
        closeLabel={t("common.close")}
        footer={
          <div className="flex justify-end gap-2">
            <Button
              variant="secondary"
              disabled={saving}
              onClick={() => setEditorOpen(false)}
            >
              {t("common.cancel")}
            </Button>
            <Button disabled={saving} onClick={handleSave}>
              {t("common.save")}
            </Button>
          </div>
        }
      >
        <div className="space-y-3">
          <Field label={t("settings.api.fields.name")}>
            <Input
              value={draft.name}
              disabled={editingPreset}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  name: event.target.value,
                }))
              }
            />
          </Field>
          <Field label={t("settings.api.fields.baseUrl")}>
            <Input
              value={draft.base_url}
              disabled={editingPreset}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  base_url: event.target.value,
                }))
              }
            />
          </Field>
          <Field label={t("settings.api.fields.path")}>
            <Input
              value={draft.transcription_path}
              disabled={editingPreset}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  transcription_path: event.target.value,
                }))
              }
            />
          </Field>
          <Field label={t("settings.api.fields.model")}>
            <Input
              value={draft.model}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  model: event.target.value,
                }))
              }
            />
          </Field>
          <Field label={t("settings.api.fields.auth")}>
            <select
              className="px-2 py-1 text-sm bg-mid-gray/10 border border-mid-gray/80 rounded-md"
              value={draft.auth_type}
              disabled={editingPreset}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  auth_type: event.target.value as TranscriptionAuthType,
                }))
              }
            >
              {authOptions.map((option) => (
                <option key={option} value={option}>
                  {t(`settings.api.auth.${option}`)}
                </option>
              ))}
            </select>
          </Field>
          {draft.auth_type === "custom_header" && (
            <Field label={t("settings.api.fields.headerName")}>
              <Input
                value={draft.auth_header_name ?? ""}
                disabled={editingPreset}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    auth_header_name: event.target.value,
                  }))
                }
              />
            </Field>
          )}
          {draft.auth_type !== "none" && (
            <Field label={t("settings.api.fields.apiKey")}>
              <Input
                type="password"
                value={secretDraft}
                placeholder={t("settings.api.fields.apiKeyPlaceholder")}
                onChange={(event) => setSecretDraft(event.target.value)}
              />
              {draft.id && (
                <Button
                  variant="danger-ghost"
                  size="sm"
                  onClick={() =>
                    runAction(draft.id as string, () =>
                      clearTranscriptionEndpointSecret(draft.id as string),
                    )
                  }
                >
                  {t("settings.api.removeKey")}
                </Button>
              )}
            </Field>
          )}
          <Field label={t("settings.api.fields.timeout")}>
            <Input
              type="number"
              min={5}
              value={draft.timeout_secs}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  timeout_secs: Number(event.target.value) || 60,
                }))
              }
            />
          </Field>
          <Field label={t("settings.api.fields.extraJson")}>
            <Input
              value={draft.extra_params_json}
              placeholder="{}"
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  extra_params_json: event.target.value,
                }))
              }
            />
          </Field>
          <label className="flex items-center justify-between text-sm">
            <span className="text-text/70">
              {t("settings.api.fields.sendLanguage")}
            </span>
            <input
              type="checkbox"
              checked={draft.send_language}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  send_language: event.target.checked,
                }))
              }
            />
          </label>
        </div>
      </Dialog>
    </div>
  );
};

const Field: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <label className="flex flex-col gap-1 text-sm">
    <span className="text-text/70">{label}</span>
    {children}
  </label>
);
