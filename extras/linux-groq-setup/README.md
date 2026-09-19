# Configuração pessoal do Handy no Ubuntu

Este diretório preserva a configuração usada com o endpoint de transcrição da
Groq e o script de colagem compatível com terminais e aplicativos gráficos.

O backup não contém a chave da API Groq, histórico, gravações ou logs.

## Restauração

1. Instale esta versão do Handy.
2. Abra o Handy uma vez e feche-o completamente.
3. Instale as dependências:

   ```bash
   sudo apt update
   sudo apt install jq xdotool x11-utils xsel coreutils
   ```

4. Execute:

   ```bash
   chmod +x extras/linux-groq-setup/restaurar-handy.sh
   ./extras/linux-groq-setup/restaurar-handy.sh
   ```

5. Abra o Handy e insira novamente a chave da API Groq.
6. Teste o atalho `Ctrl esquerdo + Espaço`.

O restaurador ativa o microfone permanente para reduzir a latência inicial,
seleciona português, Groq com `whisper-large-v3-turbo` e instala a colagem
inteligente: `Ctrl+Shift+V` em terminais e `Ctrl+V` nos demais aplicativos.

Antes de alterar as configurações, ele cria uma cópia com data e hora. Se o
nome do dispositivo mudar depois da reinstalação, selecione o Fifine novamente.
