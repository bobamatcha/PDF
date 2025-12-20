//! Embedding model integration using Hugging Face Candle
//!
//! Provides local embedding generation using the BGE-M3 model.

use anyhow::{anyhow, Result};
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::{layer_norm, linear, Activation, LayerNorm, Linear, VarBuilder};
use serde::Deserialize;
use std::path::Path;
use tokenizers::Tokenizer;

/// Embedding dimension for BGE-M3 model
pub const EMBEDDING_DIM: usize = 1024;

/// Maximum sequence length for the model
pub const MAX_SEQ_LEN: usize = 8192;

/// Model configuration loaded from config.json
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub intermediate_size: usize,
    pub hidden_act: String,
    pub hidden_dropout_prob: f64,
    pub attention_probs_dropout_prob: f64,
    pub max_position_embeddings: usize,
    pub type_vocab_size: usize,
    pub layer_norm_eps: f64,
    pub pad_token_id: usize,
    #[serde(default)]
    pub position_embedding_type: String,
    #[serde(default)]
    pub use_cache: bool,
}

impl Default for Config {
    fn default() -> Self {
        // BGE-M3 default configuration (XLM-RoBERTa based)
        Self {
            vocab_size: 250002,
            hidden_size: 1024,
            num_hidden_layers: 24,
            num_attention_heads: 16,
            intermediate_size: 4096,
            hidden_act: "gelu".to_string(),
            hidden_dropout_prob: 0.1,
            attention_probs_dropout_prob: 0.1,
            max_position_embeddings: 8194,
            type_vocab_size: 1,
            layer_norm_eps: 1e-5,
            pad_token_id: 1,
            position_embedding_type: "absolute".to_string(),
            use_cache: true,
        }
    }
}

/// Embedding model wrapper for BGE-M3
pub struct EmbeddingModel {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

/// BERT Embeddings layer
struct BertEmbeddings {
    word_embeddings: candle_nn::Embedding,
    position_embeddings: candle_nn::Embedding,
    token_type_embeddings: candle_nn::Embedding,
    layer_norm: LayerNorm,
    dropout: candle_nn::Dropout,
}

impl BertEmbeddings {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let word_embeddings = candle_nn::embedding(
            config.vocab_size,
            config.hidden_size,
            vb.pp("word_embeddings"),
        )?;
        let position_embeddings = candle_nn::embedding(
            config.max_position_embeddings,
            config.hidden_size,
            vb.pp("position_embeddings"),
        )?;
        let token_type_embeddings = candle_nn::embedding(
            config.type_vocab_size,
            config.hidden_size,
            vb.pp("token_type_embeddings"),
        )?;
        let layer_norm = layer_norm(
            config.hidden_size,
            config.layer_norm_eps,
            vb.pp("LayerNorm"),
        )?;
        let dropout = candle_nn::Dropout::new(config.hidden_dropout_prob as f32);

        Ok(Self {
            word_embeddings,
            position_embeddings,
            token_type_embeddings,
            layer_norm,
            dropout,
        })
    }

    fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor, position_ids: &Tensor) -> Result<Tensor> {
        let word_embeds = self.word_embeddings.forward(input_ids)?;
        let position_embeds = self.position_embeddings.forward(position_ids)?;
        let token_type_embeds = self.token_type_embeddings.forward(token_type_ids)?;

        let embeddings = ((word_embeds + position_embeds)? + token_type_embeds)?;
        let embeddings = self.layer_norm.forward(&embeddings)?;
        let embeddings = self.dropout.forward(&embeddings, false)?;

        Ok(embeddings)
    }
}

/// BERT Self-Attention layer
struct BertSelfAttention {
    query: Linear,
    key: Linear,
    value: Linear,
    dropout: candle_nn::Dropout,
    num_attention_heads: usize,
    attention_head_size: usize,
}

impl BertSelfAttention {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let attention_head_size = config.hidden_size / config.num_attention_heads;
        let all_head_size = config.num_attention_heads * attention_head_size;

        let query = linear(config.hidden_size, all_head_size, vb.pp("query"))?;
        let key = linear(config.hidden_size, all_head_size, vb.pp("key"))?;
        let value = linear(config.hidden_size, all_head_size, vb.pp("value"))?;
        let dropout = candle_nn::Dropout::new(config.attention_probs_dropout_prob as f32);

        Ok(Self {
            query,
            key,
            value,
            dropout,
            num_attention_heads: config.num_attention_heads,
            attention_head_size,
        })
    }

    fn transpose_for_scores(&self, x: &Tensor) -> Result<Tensor> {
        let mut new_shape = x.dims().to_vec();
        new_shape.pop();
        new_shape.push(self.num_attention_heads);
        new_shape.push(self.attention_head_size);
        Ok(x.reshape(new_shape)?.transpose(1, 2)?)
    }

    fn forward(&self, hidden_states: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        let query_layer = self.query.forward(hidden_states)?;
        let key_layer = self.key.forward(hidden_states)?;
        let value_layer = self.value.forward(hidden_states)?;

        let query_layer = self.transpose_for_scores(&query_layer)?;
        let key_layer = self.transpose_for_scores(&key_layer)?;
        let value_layer = self.transpose_for_scores(&value_layer)?;

        // Compute attention scores
        let attention_scores = query_layer.matmul(&key_layer.t()?)?;
        let attention_scores = (attention_scores / (self.attention_head_size as f64).sqrt())?;

        // Apply attention mask
        let attention_scores = attention_scores.broadcast_add(attention_mask)?;

        // Normalize to probabilities
        let attention_probs = candle_nn::ops::softmax_last_dim(&attention_scores)?;
        let attention_probs = self.dropout.forward(&attention_probs, false)?;

        // Apply attention to values
        let context_layer = attention_probs.matmul(&value_layer)?;
        let context_layer = context_layer.transpose(1, 2)?.contiguous()?;

        let mut new_shape = context_layer.dims().to_vec();
        new_shape.pop();
        new_shape.pop();
        new_shape.push(self.num_attention_heads * self.attention_head_size);

        Ok(context_layer.reshape(new_shape)?)
    }
}

/// BERT Self-Attention output
struct BertSelfOutput {
    dense: Linear,
    layer_norm: LayerNorm,
    dropout: candle_nn::Dropout,
}

impl BertSelfOutput {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let dense = linear(config.hidden_size, config.hidden_size, vb.pp("dense"))?;
        let layer_norm = layer_norm(
            config.hidden_size,
            config.layer_norm_eps,
            vb.pp("LayerNorm"),
        )?;
        let dropout = candle_nn::Dropout::new(config.hidden_dropout_prob as f32);

        Ok(Self {
            dense,
            layer_norm,
            dropout,
        })
    }

    fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> Result<Tensor> {
        let hidden_states = self.dense.forward(hidden_states)?;
        let hidden_states = self.dropout.forward(&hidden_states, false)?;
        Ok(self.layer_norm.forward(&(hidden_states + input_tensor)?)?)
    }
}

/// BERT Attention (combines self-attention and output)
struct BertAttention {
    self_attention: BertSelfAttention,
    self_output: BertSelfOutput,
}

impl BertAttention {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let self_attention = BertSelfAttention::load(vb.pp("self"), config)?;
        let self_output = BertSelfOutput::load(vb.pp("output"), config)?;

        Ok(Self {
            self_attention,
            self_output,
        })
    }

    fn forward(&self, hidden_states: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        let self_output = self.self_attention.forward(hidden_states, attention_mask)?;
        self.self_output.forward(&self_output, hidden_states)
    }
}

/// BERT Intermediate layer (Feed-Forward Network first part)
struct BertIntermediate {
    dense: Linear,
    activation: Activation,
}

impl BertIntermediate {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let dense = linear(config.hidden_size, config.intermediate_size, vb.pp("dense"))?;
        let activation = match config.hidden_act.as_str() {
            "gelu" => Activation::Gelu,
            "relu" => Activation::Relu,
            _ => Activation::Gelu,
        };

        Ok(Self { dense, activation })
    }

    fn forward(&self, hidden_states: &Tensor) -> Result<Tensor> {
        let hidden_states = self.dense.forward(hidden_states)?;
        Ok(self.activation.forward(&hidden_states)?)
    }
}

/// BERT Output layer (Feed-Forward Network second part)
struct BertOutput {
    dense: Linear,
    layer_norm: LayerNorm,
    dropout: candle_nn::Dropout,
}

impl BertOutput {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let dense = linear(config.intermediate_size, config.hidden_size, vb.pp("dense"))?;
        let layer_norm = layer_norm(
            config.hidden_size,
            config.layer_norm_eps,
            vb.pp("LayerNorm"),
        )?;
        let dropout = candle_nn::Dropout::new(config.hidden_dropout_prob as f32);

        Ok(Self {
            dense,
            layer_norm,
            dropout,
        })
    }

    fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> Result<Tensor> {
        let hidden_states = self.dense.forward(hidden_states)?;
        let hidden_states = self.dropout.forward(&hidden_states, false)?;
        Ok(self.layer_norm.forward(&(hidden_states + input_tensor)?)?)
    }
}

/// BERT Layer (one transformer block)
struct BertLayer {
    attention: BertAttention,
    intermediate: BertIntermediate,
    output: BertOutput,
}

impl BertLayer {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let attention = BertAttention::load(vb.pp("attention"), config)?;
        let intermediate = BertIntermediate::load(vb.pp("intermediate"), config)?;
        let output = BertOutput::load(vb.pp("output"), config)?;

        Ok(Self {
            attention,
            intermediate,
            output,
        })
    }

    fn forward(&self, hidden_states: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        let attention_output = self.attention.forward(hidden_states, attention_mask)?;
        let intermediate_output = self.intermediate.forward(&attention_output)?;
        self.output.forward(&intermediate_output, &attention_output)
    }
}

/// BERT Encoder (stack of transformer layers)
struct BertEncoder {
    layers: Vec<BertLayer>,
}

impl BertEncoder {
    fn load(vb: VarBuilder, config: &Config) -> Result<Self> {
        let mut layers = Vec::with_capacity(config.num_hidden_layers);
        let vb_l = vb.pp("layer");

        for i in 0..config.num_hidden_layers {
            let layer = BertLayer::load(vb_l.pp(i), config)?;
            layers.push(layer);
        }

        Ok(Self { layers })
    }

    fn forward(&self, hidden_states: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        let mut hidden_states = hidden_states.clone();

        for layer in &self.layers {
            hidden_states = layer.forward(&hidden_states, attention_mask)?;
        }

        Ok(hidden_states)
    }
}

/// Full BERT model for embeddings
struct BertModel {
    embeddings: BertEmbeddings,
    encoder: BertEncoder,
    config: Config,
    device: Device,
}

impl BertModel {
    fn load(vb: VarBuilder, config: Config, device: Device) -> Result<Self> {
        let embeddings = BertEmbeddings::load(vb.pp("embeddings"), &config)?;
        let encoder = BertEncoder::load(vb.pp("encoder"), &config)?;

        Ok(Self {
            embeddings,
            encoder,
            config,
            device,
        })
    }

    fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor, position_ids: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        let embedding_output = self.embeddings.forward(input_ids, token_type_ids, position_ids)?;
        let sequence_output = self.encoder.forward(&embedding_output, attention_mask)?;
        Ok(sequence_output)
    }
}

impl EmbeddingModel {
    /// Load the BGE-M3 embedding model from a local path
    ///
    /// # Arguments
    /// * `model_path` - Path to the model directory containing:
    ///   - `tokenizer.json` - The tokenizer configuration
    ///   - `model.safetensors` - The model weights
    ///   - `config.json` - The model configuration
    pub async fn load(model_path: &Path) -> Result<Self> {
        // Determine device (prefer CUDA if available)
        let device = if candle_core::utils::cuda_is_available() {
            Device::new_cuda(0)?
        } else if candle_core::utils::metal_is_available() {
            Device::new_metal(0)?
        } else {
            Device::Cpu
        };

        tracing::info!("Loading embedding model on device: {:?}", device);

        // Load configuration
        let config_path = model_path.join("config.json");
        if !config_path.exists() {
            return Err(anyhow!(
                "Config not found at {}",
                config_path.display()
            ));
        }
        let config_str = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| anyhow!("Failed to parse config.json: {}", e))?;

        tracing::info!(
            "Loaded config: {} layers, {} hidden size, {} attention heads",
            config.num_hidden_layers,
            config.hidden_size,
            config.num_attention_heads
        );

        // Load tokenizer
        let tokenizer_path = model_path.join("tokenizer.json");
        if !tokenizer_path.exists() {
            return Err(anyhow!(
                "Tokenizer not found at {}",
                tokenizer_path.display()
            ));
        }
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        // Load model weights from safetensors
        let weights_path = model_path.join("model.safetensors");
        if !weights_path.exists() {
            return Err(anyhow!(
                "Model weights not found at {}",
                weights_path.display()
            ));
        }

        tracing::info!("Loading model weights from {}", weights_path.display());

        // Create VarBuilder from safetensors file
        let dtype = DType::F32;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], dtype, &device)?
        };

        // Load the BERT model structure
        // BGE-M3 uses XLM-RoBERTa which has a "roberta" prefix in the state dict
        let vb = vb.pp("roberta");
        let model = BertModel::load(vb, config, device.clone())?;

        tracing::info!("Model loaded successfully");

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// Download the BGE-M3 model from Hugging Face Hub
    pub async fn download(cache_dir: &Path) -> Result<std::path::PathBuf> {
        use hf_hub::api::sync::Api;

        tracing::info!("Downloading BGE-M3 model from Hugging Face Hub...");

        let api = Api::new()?;
        let repo = api.model("BAAI/bge-m3".to_string());

        // Download required files
        let _tokenizer = repo.get("tokenizer.json")?;
        let _config = repo.get("config.json")?;
        let _weights = repo.get("model.safetensors")?;

        // Return the cache directory where files are stored
        let model_path = cache_dir.join("BAAI/bge-m3");
        Ok(model_path)
    }

    /// Generate embedding for a single text
    ///
    /// # Arguments
    /// * `text` - The text to embed
    ///
    /// # Returns
    /// A 1024-dimensional embedding vector
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize input
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        let input_ids: Vec<u32> = encoding.get_ids().to_vec();
        let attention_mask: Vec<u32> = encoding.get_attention_mask().to_vec();

        // Truncate if necessary
        let seq_len = input_ids.len().min(MAX_SEQ_LEN);
        let input_ids: Vec<u32> = input_ids[..seq_len].to_vec();
        let attention_mask: Vec<u32> = attention_mask[..seq_len].to_vec();

        // Create tensors
        let input_ids = Tensor::new(&input_ids[..], &self.device)?;
        let attention_mask = Tensor::new(&attention_mask[..], &self.device)?;

        // Create token type IDs (all zeros for single sentence)
        let token_type_ids = Tensor::zeros(seq_len, DType::U32, &self.device)?;

        // Create position IDs (0, 1, 2, ..., seq_len-1)
        let position_ids: Vec<u32> = (0..seq_len as u32).collect();
        let position_ids = Tensor::new(&position_ids[..], &self.device)?;

        // Forward pass through model
        let output = self.forward(&input_ids, &token_type_ids, &position_ids, &attention_mask)?;

        // Mean pooling
        let pooled = self.mean_pooling(&output, &attention_mask)?;

        // L2 normalize
        let normalized = self.l2_normalize(&pooled)?;

        // Convert to Vec<f32>
        Ok(normalized.to_vec1::<f32>()?)
    }

    /// Generate embeddings for multiple texts (batched for efficiency)
    ///
    /// # Arguments
    /// * `texts` - Slice of texts to embed
    ///
    /// # Returns
    /// Vector of 1024-dimensional embedding vectors
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // For now, process sequentially
        // TODO: Implement proper batching in Phase 2.1
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Forward pass through the model
    fn forward(&self, input_ids: &Tensor, token_type_ids: &Tensor, position_ids: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        // Prepare attention mask for multi-head attention
        // Convert from [seq_len] to [1, 1, seq_len, seq_len]
        let attention_mask = self.prepare_attention_mask(attention_mask)?;

        // Forward through the model
        let output = self.model.forward(input_ids, token_type_ids, position_ids, &attention_mask)?;
        Ok(output)
    }

    /// Prepare attention mask for multi-head attention
    /// Converts from [seq_len] to extended attention mask with -inf for masked positions
    fn prepare_attention_mask(&self, attention_mask: &Tensor) -> Result<Tensor> {
        let seq_len = attention_mask.dim(0)?;

        // Convert to f32 and reshape to [1, seq_len]
        let mask = attention_mask.to_dtype(DType::F32)?;
        let mask = mask.unsqueeze(0)?;

        // Expand to [1, 1, seq_len, seq_len]
        let mask = mask.unsqueeze(0)?;
        let mask = mask.unsqueeze(0)?;

        // Broadcast to create causal mask shape
        let ones = Tensor::ones((seq_len, seq_len), DType::F32, &self.device)?;
        let mask = mask.broadcast_mul(&ones.unsqueeze(0)?.unsqueeze(0)?)?;

        // Convert to additive mask (0 for attend, -inf for ignore)
        // (1 - mask) * -10000 gives -10000 for masked positions, 0 for valid positions
        let mask = mask.affine(-1.0, 1.0)?;
        let mask = (mask * -10000.0)?;

        Ok(mask)
    }

    /// Apply mean pooling over the sequence dimension
    fn mean_pooling(&self, output: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        // output shape: [seq_len, hidden_size]
        // attention_mask shape: [seq_len]

        // Convert attention mask to f32 and expand to [seq_len, 1]
        let mask = attention_mask.to_dtype(DType::F32)?;
        let mask = mask.unsqueeze(1)?;

        // Multiply output by mask to zero out padding
        let masked_output = output.broadcast_mul(&mask)?;

        // Sum along sequence dimension
        let sum = masked_output.sum(0)?;

        // Count non-padding tokens
        let count = mask.sum(0)?;

        // Divide to get mean
        let mean = sum.broadcast_div(&count)?;

        Ok(mean)
    }

    /// L2 normalize a tensor
    fn l2_normalize(&self, tensor: &Tensor) -> Result<Tensor> {
        // Compute L2 norm
        let norm = tensor.sqr()?.sum_all()?.sqrt()?;

        // Add small epsilon to avoid division by zero
        let epsilon = 1e-12;
        let norm = (norm + epsilon)?;

        // Normalize
        Ok(tensor.broadcast_div(&norm)?)
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        EMBEDDING_DIM
    }

    /// Get the device the model is running on
    pub fn device(&self) -> &Device {
        &self.device
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dimension() {
        assert_eq!(EMBEDDING_DIM, 1024);
    }
}
