# LlamaEdge-Search API Server

<!-- @import "[TOC]" {cmd="toc" depthFrom=1 depthTo=6 orderedList=false} -->

<!-- code_chunk_output -->

- [LlamaEdge-RAG API Server](#llamaedge-rag-api-server)
  - [Introduction](#introduction)
    - [Endpoints](#endpoints)
      - [`/v1/models` endpoint](#v1models-endpoint)
      - [`/v1/chat/completions` endpoint](#v1chatcompletions-endpoint)
      - [`/v1/files` endpoint](#v1files-endpoint)
      - [`/v1/chunks` endpoint](#v1chunks-endpoint)
      - [`/v1/embeddings` endpoint](#v1embeddings-endpoint)
      - [`/v1/info` endpoint](#v1info-endpoint)
  - [Setup](#setup)
  - [Build](#build)
  - [Execute](#execute)
  - [Modify](#modify)
<!-- /code_chunk_output -->

## Introduction

LlamaEdge-Search API server is an extension of the Llama API Server with search capabilties. This server will run every query against search results obtained from the internet. The server is implemented in WebAssembly (Wasm) and runs on [WasmEdge Runtime](https://github.com/WasmEdge/WasmEdge).

### Endpoints

#### `/v1/models` endpoint

`rag-api-server` provides a POST API `/v1/models` to list currently available models.

<details> <summary> Example </summary>

You can use `curl` to test it on a new terminal:

```bash
curl -X POST http://localhost:8080/v1/models -H 'accept:application/json'
```

If the command runs successfully, you should see the similar output as below in your terminal:

```json
{
  "object": "list",
  "data": [
    {
      "id": "Llama-2-7b-chat-hf-Q5_K_M",
      "created": 1721824510,
      "object": "model",
      "owned_by": "Not specified"
    }
  ]
}
```

</details>

#### `/v1/chat/completions` endpoint

Ask a question using OpenAI's JSON message format.

<details> <summary> Example </summary>

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
    -H 'accept:application/json' \
    -H 'Content-Type: application/json' \
    -d '{"messages":[{"role":"system", "content": "You are a helpful assistant."}, {"role":"user", "content": "Who is Robert Oppenheimer?"}], "model":"Llama-2-7b-chat-hf-Q5_K_M"}'
```

Here is the response.

```json
{
    "id":"",
    "object":"chat.completion",
    "created":1697092593,
    "model":"llama-2-chat",
    "choices":[
        {
            "index":0,
            "message":{
                "role":"assistant",
                "content":"Ah, a most excellent question! Robert Oppenheimer (1904-1967) was an American theoretical physicist and director of the Manhattan Project, the secret research and development project that produced the atomic bomb during World War II. He is widely regarded as one of the most important physicists of the 20th century.\n\nOppenheimer was born in New York City and grew up in a family of intellectuals. He studied physics at Harvard University, where he earned his undergraduate degree, and later at Cambridge University, where he earned his PhD. After completing his education, he worked at several universities and research institutions, including the University of California, Berkeley, and Princeton University.\n\nOppenheimer's most significant contribution to physics was his work on quantum mechanics, particularly his development of the theory of quantum field theory. He also made important contributions to the study of nuclear physics and was one of the leaders of the Manhattan Project, which produced the atomic bomb during World War II.\n\nDespite his many accomplishments in physics, Oppenheimer is perhaps best known for his role in the development of the atomic bomb. He was a strong advocate for international cooperation on nuclear weapons and later became a vocal critic of the arms race between the United States and the Soviet Union.\n\nOppenheimer's life was marked by both personal and professional struggles. He was openly gay, which was illegal at the time, and he struggled with alcoholism and depression throughout his life. Despite these challenges, he remained a brilliant physicist and a passionate advocate for peaceful uses of nuclear energy until his death in 1967.\n\nToday, Oppenheimer is remembered as one of the most influential scientists of the 20th century, and his legacy continues to inspire new generations of physicists and peace activists around the world."
            },
            "finish_reason":"stop"
        }
    ],
    "usage":{
        "prompt_tokens":9,
        "completion_tokens":12,
        "total_tokens":21
    }
}
```

</details>

#### `/v1/files` endpoint

Upload files to to chunk them and compute their embeddings.

<details> <summary> Example </summary>

The following command upload a text file [paris.txt](https://huggingface.co/datasets/gaianet/paris/raw/main/paris.txt) to the API server via the `/v1/files` endpoint:

```bash
curl -X POST http://localhost:8080/v1/files -F "file=@paris.txt"
```

If the command is successful, you should see the similar output as below in your terminal:

```json
{
    "id": "file_4bc24593-2a57-4646-af16-028855e7802e",
    "bytes": 2161,
    "created_at": 1711611801,
    "filename": "paris.txt",
    "object": "file",
    "purpose": "assistants"
}
```

The `id` and `filename` fields are important for the next step, for example, to segment the uploaded file to chunks for computing embeddings.

</details>

#### `/v1/chunks` endpoint

To segment the uploaded file to chunks for computing embeddings, use the `/v1/chunks` API.

<details> <summary> Example </summary>

The following command sends the uploaded file ID and filename to the API server and gets the chunks:

```bash
curl -X POST http://localhost:8080/v1/chunks \
    -H 'accept:application/json' \
    -H 'Content-Type: application/json' \
    -d '{"id":"file_4bc24593-2a57-4646-af16-028855e7802e", "filename":"paris.txt", "chunk_capacity":100}'
```

The following is an example return with the generated chunks:

```json
{
    "id": "file_4bc24593-2a57-4646-af16-028855e7802e",
    "filename": "paris.txt",
    "chunks": [
        "Paris, city and capital of France, ..., for Paris has retained its importance as a centre for education and intellectual pursuits.",
        "Paris’s site at a crossroads ..., drawing to itself much of the talent and vitality of the provinces."
    ]
}
```

</details>

#### `/v1/embeddings` endpoint

To compute embeddings for user query or file chunks, use the `/v1/embeddings` API.

<details> <summary> Example </summary>

The following command sends a query to the API server and gets the embeddings as return:

```bash
curl -X POST http://localhost:8080/v1/embeddings \
    -H 'accept:application/json' \
    -H 'Content-Type: application/json' \
    -d '{"model": "e5-mistral-7b-instruct-Q5_K_M", "input":["Paris, city and capital of France, ..., for Paris has retained its importance as a centre for education and intellectual pursuits.", "Paris’s site at a crossroads ..., drawing to itself much of the talent and vitality of the provinces."]}'
```

The embeddings returned are like below:

```json
{
    "object": "list",
    "data": [
        {
            "index": 0,
            "object": "embedding",
            "embedding": [
                0.1428378969,
                -0.0447309874,
                0.007660218049,
                ...
                -0.0128974719,
                -0.03543198109,
                0.03974733502,
                0.00946635101,
                -0.01531364303
            ]
        },
        {
            "index": 1,
            "object": "embedding",
            "embedding": [
                0.0697753951,
                -0.0001159032545,
                0.02073983476,
                ...
                0.03565846011,
                -0.04550019652,
                0.02691745944,
                0.02498772368,
                -0.003226313973
            ]
        }
    ],
    "model": "e5-mistral-7b-instruct-Q5_K_M",
    "usage": {
        "prompt_tokens": 491,
        "completion_tokens": 0,
        "total_tokens": 491
    }
}
```

</details>

#### `/v1/info` endpoint

`/v1/info` endpoint provides the information of the API server, including the version of the server, the parameters of models, and etc.

<details> <summary> Example </summary>

You can use `curl` to test it on a new terminal:

```bash
curl -X POST http://localhost:8080/v1/info -H 'accept:application/json'
```

If the command runs successfully, you should see the similar output as below in your terminal:

```json
{
  "api_server": {
    "type": "llama",
    "version": "0.1.0",
    "ggml_plugin_version": "b3405 (commit 5e116e8d)",
    "port": "8080"
  },
  "chat_model": {
    "name": "Llama-2-7b-chat-hf-Q5_K_M",
    "type": "chat",
    "ctx_size": 4096,
    "batch_size": 512,
    "prompt_template": "Llama2Chat",
    "n_predict": 1024,
    "n_gpu_layers": 100,
    "temperature": 1.0,
    "top_p": 1.0,
    "repeat_penalty": 1.1,
    "presence_penalty": 0.0,
    "frequency_penalty": 0.0
  },
  "embedding_model": {
    "name": "all-MiniLM-L6-v2-ggml-model-f16",
    "type": "embedding",
    "ctx_size": 384,
    "batch_size": 512
  },
  "extras": {}
}
```
</details>

## Setup

Llama-RAG API server runs on WasmEdge Runtime. According to the operating system you are using, choose the installation command:

<details> <summary> For macOS (apple silicon) </summary>

```console
# install WasmEdge-0.13.4 with wasi-nn-ggml plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugin wasi_nn-ggml

# Assuming you use zsh (the default shell on macOS), run the following command to activate the environment
source $HOME/.zshenv
```

</details>

<details> <summary> For Ubuntu (>= 20.04) </summary>

```console
# install libopenblas-dev
apt update && apt install -y libopenblas-dev

# install WasmEdge-0.13.4 with wasi-nn-ggml plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugin wasi_nn-ggml

# Assuming you use bash (the default shell on Ubuntu), run the following command to activate the environment
source $HOME/.bashrc
```

</details>

<details> <summary> For General Linux </summary>

```console
# install WasmEdge-0.13.4 with wasi-nn-ggml plugin
curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugin wasi_nn-ggml

# Assuming you use bash (the default shell on Ubuntu), run the following command to activate the environment
source $HOME/.bashrc
```

</details>

## Build

```bash
# Clone the repository
git clone https://github.com/suryyyansh/search-api-server.git

# Change the working directory
cd search-api-server

# Build `search-api-server.wasm` with the `http` support only, or
cargo build --target wasm32-wasi --release

# Copy the `search-api-server.wasm` to the root directory
cp target/wasm32-wasi/release/search-api-server.wasm .
```

<details> <summary> To check the CLI options, </summary>

To check the CLI options of the `search-api-server` wasm app, you can run the following command:

  ```bash
  $ wasmedge search-api-server.wasm -h
    
  LlamaEdge-Search API Server

  Usage: search-api-server.wasm [OPTIONS] --prompt-template <PROMPT_TEMPLATE>

  Options:
    -m, --model-name <MODEL_NAME>
            Sets names for chat and/or embedding models. To run both chat and embedding models, the names should be separated by comma without space, for example, '--model-name Llama-2-7b,all-minilm'. The first value is for the chat model, and the second is for the embedding model [default: default]
    -a, --model-alias <MODEL_ALIAS>
            Model aliases for chat and embedding models [default: default,embedding]
    -c, --ctx-size <CTX_SIZE>
            Sets context sizes for chat and/or embedding models. To run both chat and embedding models, the sizes should be separated by comma without space, for example, '--ctx-size 4096,384'. The first value is for the chat model, and the second is for the embedding model [default: 4096,384]
    -b, --batch-size <BATCH_SIZE>
            Sets batch sizes for chat and/or embedding models. To run both chat and embedding models, the sizes should be separated by comma without space, for example, '--batch-size 128,64'. The first value is for the chat model, and the second is for the embedding model [default: 512,512]
    -p, --prompt-template <PROMPT_TEMPLATE>
            Sets prompt templates for chat and/or embedding models, respectively. To run both chat and embedding models, the prompt templates should be separated by comma without space, for example, '--prompt-template llama-2-chat,embedding'. The first value is for the chat model, and the second is for the embedding model [possible values: llama-2-chat, llama-3-chat, mistral-instruct, mistral-tool, mistrallite, openchat, codellama-instruct, codellama-super-instruct, human-assistant, vicuna-1.0-chat, vicuna-1.1-chat, vicuna-llava, chatml, chatml-tool, baichuan-2, wizard-coder, zephyr, stablelm-zephyr, intel-neural, deepseek-chat, deepseek-coder, deepseek-chat-2, solar-instruct, phi-2-chat, phi-2-instruct, phi-3-chat, phi-3-instruct, gemma-instruct, octopus, glm-4-chat, groq-llama3-tool, embedding]
    -r, --reverse-prompt <REVERSE_PROMPT>
            Halt generation at PROMPT, return control
    -n, --n-predict <N_PREDICT>
            Number of tokens to predict [default: 1024]
    -g, --n-gpu-layers <N_GPU_LAYERS>
            Number of layers to run on the GPU [default: 100]
        --no-mmap <NO_MMAP>
            Disable memory mapping for file access of chat models [possible values: true, false]
        --temp <TEMP>
            Temperature for sampling [default: 1.0]
        --top-p <TOP_P>
            An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top_p probability mass. 1.0 = disabled [default: 1.0]
        --repeat-penalty <REPEAT_PENALTY>
            Penalize repeat sequence of tokens [default: 1.1]
        --presence-penalty <PRESENCE_PENALTY>
            Repeat alpha presence penalty. 0.0 = disabled [default: 0.0]
        --frequency-penalty <FREQUENCY_PENALTY>
            Repeat alpha frequency penalty. 0.0 = disabled [default: 0.0]
        --llava-mmproj <LLAVA_MMPROJ>
            Path to the multimodal projector file
        --socket-addr <SOCKET_ADDR>
            Socket address of LlamaEdge API Server instance [default: 0.0.0.0:8080]
        --web-ui <WEB_UI>
            Root path for the Web UI files [default: chatbot-ui]
        --log-prompts
            Deprecated. Print prompt strings to stdout
        --log-stat
            Deprecated. Print statistics to stdout
        --log-all
            Deprecated. Print all log information to stdout
        --enable-rag
            Whether to enable RAG functionality (currently unimplemented)
        --max-search-results <MAX_SEARCH_RESULTS>
            Whether to enable RAG functionality (currently unimplemented) [default: 5]
        --clip-every-result <CLIP_EVERY_RESULT>
            size to clip every result to [default: 225]
        --api-key <API_KEY>
            Whether to enable RAG functionality (currently unimplemented) [default: ]
    -h, --help
            Print help
    -V, --version
            Print version
  ```

</details>

## Execute

LlamaEdge-Search API server supports 2 models: chat and embedding. The chat model is used for generating responses to user queries, while the embedding model is used for computing embeddings for user queries or file chunks. **The Search API Server requires at least a `chat` model**

For the purpose of demonstration, we use the [Llama-2-7b-chat-hf-Q5_K_M.gguf](https://huggingface.co/second-state/Llama-2-7B-Chat-GGUF/resolve/main/Llama-2-7b-chat-hf-Q5_K_M.gguf) chat model as an example. Download this model and place it in the root directory of the repository.

- Start an instance of LlamaEdge-Search API server

  ```bash
  wasmedge --dir .:.  --env LLAMA_LOG="info" \
    --nn-preload default:GGML:AUTO:Llama-2-7b-chat-hf-Q5_K_M.gguf \
    search-api-server.wasm \
    --ctx-size 4096,384 \
    --prompt-template llama-2-chat \
    --model-name Llama-2-7b-chat-hf-Q5_K_M \
    --api-key <YOUR_API_KEY> #if required by an endpoint.
  ```
## Usage Example

- [Execute](#execute) the server

- Ask a question. Search results from the backend in use will be automatically fetched and used.

    ```bash
    curl -X POST http://localhost:8080/v1/chat/completions \
        -H 'accept:application/json' \
        -H 'Content-Type: application/json' \
        -d '{"messages":[{"role":"system", "content": "You are a helpful assistant."}, {"role":"user", "content": "What\'s the current news?"}], "model":"Llama-2-7b-chat-hf-Q5_K_M"}'
    ```

## Modify

The crux of the search-api-server is `struct SearchConfig` from the `llama-core` crate when compiled with the `search` feature.

This is how it works:

1. Decide the search API (JSON based, supports HTTP) to use. There are many out there to choose from. We currently use Tavily by default.

2. Crate a new file for the search endpoint and place it in it's own file under `search/`. Don't forget to `mod <filename>` it in `src/main.rs.`.

3. Next, we'll make a `fn` in the new file that converts the raw JSON output of the search endpoint for a given query to a `struct SearchOutput` object.
  ```rust
  pub fn custom_search_parser(
      raw_results: &serde_json::Value,
  ) -> Result<SearchOutput, Box<dyn std::error::Error>> {
  
    // conversion logic that converts the raw results from the server into a SearchOutput. 
    let search_output: SearchOutput {
      url: String = <assign>
      site_name: String = <assign>
      text_content: String = <assign>
    }
    Ok(search_output);
  }
  ```

4. Next, we'll define a `struct CustomSearchInput`. This `struct` must be `Serialize`-able, as the fields will be converted directly to JSON. Later, we'll pass an instance of this struct to the `&SearchConfig.perform_search()` function to actually perform the search:
  ```rust
  #[derive(Serialize)]
  struct CustomSearchInput { 
    // sample fields. Change according to your search endpoint.
    term: String,
    max_search_results: u8,
    depth: String,
    api_key: String
  }
  ```

5.

6. The search results get included into the conversation as a System Message in `fn chat_completion_handler` in `backend/ggml.rs`.
  ```rust
  let search_input = CustomSearchInput {
    // assign fields
  }
  ```

7. The we need to place the struct SearchConfig in `src/main.rs` with our own.
 ```rust
 let search_config = SearchConfig {
    // fields
    parser: custom_search_parser()
 }
 ```

8. Now, upon recompiling the server and running it, try asking the LLM a question.

*(In progress)*
