-- Populate ai_models table with popular OpenAI and OpenRouter models
-- This migration seeds the database with top-ranked affordable models from OpenRouter rankings

-- Insert OpenAI models (for direct OpenAI provider)
INSERT INTO ai_models (provider, model_name, display_name, description, context_window, is_enabled, is_free) VALUES
    ('openai', 'gpt-4o', 'GPT-4o', 'OpenAI''s most capable multimodal model ($2.50/M input, $10.00/M output)', 128000, true, false),
    ('openai', 'gpt-4o-mini', 'GPT-4o Mini', 'Fast and affordable small model ($0.15/M input, $0.60/M output)', 128000, true, false),
    ('openai', 'gpt-5-mini', 'GPT-5 Mini', 'Latest compact model with improved performance ($0.25/M input, $2.00/M output)', 128000, true, false),
    ('openai', 'o1-preview', 'O1 Preview', 'Advanced reasoning model ($15.00/M input, $60.00/M output)', 128000, false, false),
    ('openai', 'o1-mini', 'O1 Mini', 'Fast reasoning model ($3.00/M input, $12.00/M output)', 128000, false, false)
ON CONFLICT (provider, model_name) DO NOTHING;

-- Insert OpenRouter Top Ranking Models (with pricing)
INSERT INTO ai_models (provider, model_name, display_name, description, context_window, is_enabled, is_free) VALUES
    ('openrouter', 'openai/gpt-oss-20b', 'GPT OSS 20B', 'Open source 20B model ($0.02/M input, $0.1/M output)', 131072, true, false),
    ('openrouter', 'openai/gpt-oss-120b', 'GPT OSS 120B', 'Open source 120B model ($0.039/M input, $0.19/M output)', 131072, true, false),
    ('openrouter', 'openai/gpt-oss-safeguard-20b', 'GPT OSS Safeguard 20B', 'Open-source 20B safety model ($0.075/M input, $0.30/M output)', 131072, true, false),
    ('openrouter', 'google/gemini-2.5-flash-lite', 'Gemini 2.5 Flash Lite', 'Ultra-low latency and cost efficiency ($0.10/M input, $0.40/M output)', 1048576, true, false),
    ('openrouter', 'qwen/qwen3-235b-a22b-2507', 'Qwen3 235B Instruct', 'Large-scale open model ($0.071/M input, $0.463/M output)', 262144, true, false),
    ('openrouter', 'qwen/qwen3-235b-a22b-thinking-2507', 'Qwen3 235B A22B Thinking 2507', 'Advanced reasoning with thinking mode ($0.11/M input, $0.60/M output)', 262144, true, false),
    ('openrouter', 'qwen/qwen3-next-80b-a3b-instruct', 'Qwen3 Next 80B', 'Fast stable responses ($0.09/M input, $1.10/M output)', 262144, true, false),
    ('openrouter', 'deepseek/deepseek-v3.2', 'DeepSeek V3.2', 'Latest model with GPT-5 class reasoning ($0.25/M input, $0.38/M output)', 163840, true, false),
    ('openrouter', 'x-ai/grok-code-fast-1', 'Grok Code Fast 1', 'Fast coding model ($0.20/M input, $1.50/M output)', 256000, true, false),
    ('openrouter', 'x-ai/grok-4-fast', 'Grok 4 Fast', 'SOTA cost-efficiency with 2M context ($0.20/M input, $0.50/M output)', 2000000, true, false),
    ('openrouter', 'x-ai/grok-4.1-fast', 'Grok 4.1 Fast', 'Latest multimodal with 2M context (≤128K: $0.20/$0.50 | >128K: $0.40/$1.00)', 2000000, true, false),
    ('openrouter', 'minimax/minimax-m2.1', 'MiniMax M2.1', 'Lightweight coding model with 10B activated params ($0.27/M input, $1.10/M output)', 196608, true, false),
    ('openrouter', 'z-ai/glm-4.5-air', 'GLM 4.5 Air', 'Lightweight MoE for agent-centric apps ($0.05/M input, $0.22/M output)', 131072, true, false),
    ('openrouter', 'z-ai/glm-4.7-flash', 'GLM 4.7 Flash', '30B-class SOTA optimized for agentic coding ($0.07/M input, $0.40/M output)', 200000, true, false),
    ('openrouter', 'z-ai/glm-4.6v', 'GLM 4.6V', 'Large multimodal model for visual understanding ($0.30/M input, $0.90/M output)', 131072, true, false),
    ('openrouter', 'z-ai/glm-4.7', 'GLM 4.7', 'Latest flagship with enhanced programming ($0.40/M input, $1.50/M output)', 202752, true, false),
    ('openrouter', 'google/gemini-2.5-flash', 'Gemini 2.5 Flash', 'State-of-the-art workhorse model ($0.30/M input, $2.50/M output)', 1048576, true, false),
    ('openrouter', 'moonshotai/kimi-k2.5', 'Kimi K2.5', 'Native multimodal model with 262K context ($0.50/M input, $2.80/M output)', 262144, true, false),
    ('openrouter', 'google/gemini-3-flash-preview', 'Gemini 3 Flash Preview', 'High-speed thinking model ($0.50/M input, $3/M output)', 1048576, true, false),
    ('openrouter', 'xiaomi/mimo-v2-flash', 'MiMo V2 Flash', '262K context ($0.09/M input, $0.29/M output)', 262144, true, false),
    ('openrouter', 'mistralai/mistral-small-creative', 'Mistral Small Creative', 'Experimental small model for creative writing and roleplay ($0.10/M input, $0.30/M output)', 33000, true, false),
    ('openrouter', 'mistralai/devstral-2512', 'Devstral 2512', '123B SOTA open-source model for agentic coding ($0.05/M input, $0.22/M output)', 262144, true, false),
    ('openrouter', 'mistralai/ministral-14b-2512', 'Ministral 14B 2512', 'Frontier capabilities with vision, 262K context ($0.20/M input, $0.20/M output)', 262144, true, false),
    ('openrouter', 'arcee-ai/trinity-mini', 'Trinity Mini', '26B MoE with 3B active, efficient reasoning ($0.045/M input, $0.15/M output)', 131072, true, false),
    ('openrouter', 'qwen/qwen3-vl-30b-a3b-instruct', 'Qwen3 VL 30B A3B Instruct', 'Multimodal model with 262K context ($0.15/M input, $0.60/M output)', 262144, true, false)
ON CONFLICT (provider, model_name) DO NOTHING;

-- Insert FREE models from OpenRouter (no API costs)
INSERT INTO ai_models (provider, model_name, display_name, description, context_window, is_enabled, is_free) VALUES
    ('openrouter', 'deepseek/deepseek-r1-0528:free', 'DeepSeek R1 0528 (FREE)', '671B params, performance on par with OpenAI o1, fully open-source (FREE)', 164000, true, true),
    ('openrouter', 'stepfun/step-3.5-flash:free', 'Step 3.5 Flash (FREE)', 'StepFun Step 3.5 Flash with 256k context (FREE)', 256000, true, true),
    ('openrouter', 'z-ai/glm-4.5-air:free', 'GLM 4.5 Air (FREE)', 'Z.AI GLM 4.5 Air with 131k context (FREE)', 131072, true, true),
    ('openrouter', 'tngtech/deepseek-r1t2-chimera:free', 'DeepSeek R1T2 Chimera (FREE)', '671B MoE model with strong reasoning, 20% faster than R1 (FREE)', 164000, true, true),
    ('openrouter', 'qwen/qwen3-coder:free', 'Qwen3 Coder 480B (FREE)', '480B MoE code generation with 35B active, optimized for coding (FREE)', 262000, true, true),
    ('openrouter', 'google/gemma-3-27b-it:free', 'Gemma 3 27B (FREE)', 'Google''s open source multimodal model with 128k context (FREE)', 131000, true, true),
    ('openrouter', 'arcee-ai/trinity-large-preview:free', 'Trinity Large Preview (FREE)', 'Arcee AI 400B MoE with 13B active, frontier-scale open-weight model (FREE)', 131000, true, true),
    ('openrouter', 'arcee-ai/trinity-mini:free', 'Trinity Mini (FREE)', 'Arcee AI 26B MoE with 3B active, efficient reasoning with 128k context (FREE)', 131000, true, true)
ON CONFLICT (provider, model_name) DO NOTHING;

-- Add comment for documentation
COMMENT ON TABLE ai_models IS 'Stores all available AI models from all providers with global enable/disable control. Populated with top-ranked affordable models from OpenRouter (Feb 2026).';
