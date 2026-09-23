# Nemotron 3.5 ASR PT-BR for Vozel

This optional local model is a Q8_0 GGUF conversion of
[`ottema/nemotron-3.5-asr-ptbr`](https://huggingface.co/ottema/nemotron-3.5-asr-ptbr),
an Ottema AI fine-tune of
[`nvidia/nemotron-3.5-asr-streaming-0.6b`](https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b).
It is distinct from the multilingual Nemotron 3.5 model also listed in Vozel.

| Property          | Value                                                                                  |
| ----------------- | -------------------------------------------------------------------------------------- |
| Format            | transcribe.cpp GGUF, Q8_0                                                              |
| Filename          | `nemotron-3.5-asr-ptbr-Q8_0.gguf`                                                      |
| Size              | 751,093,920 bytes                                                                      |
| SHA-256           | `094912bc26f5f684a3809f4615d08c63c80e508432b2d9215404e37a240ac31c`                     |
| Language          | `pt-BR` only                                                                           |
| Source checkpoint | `ottema/nemotron-3.5-asr-ptbr`, HF revision `28f18acf7a1d26d088955d4de5fdb6c7ebff2e52` |
| Model license     | [OpenMDW-1.1](../../licenses/OpenMDW-1.1.txt)                                          |

The GGUF was converted locally using the `nemotron-3.5-asr-ptbr` profile in
`transcribe.cpp`'s `scripts/convert-parakeet.py`. Its header advertises the
`parakeet` architecture, `pt-BR`, streaming support and no automatic language
detection. A local CPU smoke test loaded it and transcribed a sample with an
explicit `pt-BR` language prompt. That test did not evaluate Brazilian
Portuguese accuracy or quantify conversion drift, so this distribution makes
no accuracy or speed claim for the GGUF.

The checkpoint authors' model card contains evaluation details and limitations.
Those results were obtained with NeMo and should not be assumed to apply to
this quantized GGUF without a separate evaluation.

The model is downloaded only when selected. Its weights are not committed to
the Vozel source repository. A release distributing the GGUF must also include
the OpenMDW-1.1 license and retain the NVIDIA and Ottema AI origin notices.
The companion [origin notice](NEMOTRON_PTBR_NOTICE.txt) and
`scripts/prepare-ptbr-model-release.sh` prepare those release assets and verify
the exact GGUF size and SHA-256 before upload. The model catalog expects the
four assets at the `vozel-models-v1` GitHub release in the renamed repository.
