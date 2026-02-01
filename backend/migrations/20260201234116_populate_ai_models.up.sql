-- Populate ai_models table with popular OpenAI and OpenRouter models
-- This migration seeds the database with commonly used models from both providers

-- Insert OpenAI models
INSERT INTO ai_models (provider, model_name, display_name, description, context_window, is_enabled) VALUES
    ('openai', 'gpt-4o', 'GPT-4o', 'OpenAI''s most capable multimodal model', 128000, true),
    ('openai', 'gpt-4o-mini', 'GPT-4o Mini', 'Fast and affordable small model', 128000, true),
    ('openai', 'gpt-5-mini', 'GPT-5 Mini', 'Latest compact model with improved performance', 128000, true),
    ('openai', 'o1-preview', 'O1 Preview', 'Advanced reasoning model', 128000, false),
    ('openai', 'o1-mini', 'O1 Mini', 'Fast reasoning model', 128000, false)
ON CONFLICT (provider, model_name) DO NOTHING;

-- Insert OpenRouter models (popular and high-quality models)
INSERT INTO ai_models (provider, model_name, display_name, description, context_window, is_enabled) VALUES
    -- Anthropic Claude models
    ('openrouter', 'anthropic/claude-3.5-sonnet', 'Claude 3.5 Sonnet', 'Most capable model for complex tasks', 200000, true),
    ('openrouter', 'anthropic/claude-3.5-haiku', 'Claude 3.5 Haiku', 'Fast and compact model', 200000, true),

    -- Google Gemini models
    ('openrouter', 'google/gemini-pro-1.5', 'Gemini Pro 1.5', 'Google''s flagship model with 1M context', 1000000, true),
    ('openrouter', 'google/gemini-flash-1.5', 'Gemini Flash 1.5', 'Fast model with 1M context', 1000000, true),

    -- Moonshot AI Kimi models (user requested)
    ('openrouter', 'moonshotai/kimi-k2.5', 'Kimi K2.5', 'Moonshot AI''s latest model', 128000, true),

    -- DeepSeek models (cost-effective)
    ('openrouter', 'deepseek/deepseek-chat', 'DeepSeek Chat', 'High-performance Chinese model', 128000, false),
    ('openrouter', 'deepseek/deepseek-coder', 'DeepSeek Coder', 'Specialized for coding tasks', 128000, false),

    -- Meta Llama models
    ('openrouter', 'meta-llama/llama-3.1-405b-instruct', 'Llama 3.1 405B', 'Meta''s largest open model', 128000, false),
    ('openrouter', 'meta-llama/llama-3.1-70b-instruct', 'Llama 3.1 70B', 'Balanced performance and speed', 128000, false),
    ('openrouter', 'meta-llama/llama-3.1-8b-instruct', 'Llama 3.1 8B', 'Fast small model', 128000, false),

    -- Mistral models
    ('openrouter', 'mistralai/mistral-large', 'Mistral Large', 'Mistral''s flagship model', 128000, false),
    ('openrouter', 'mistralai/mistral-medium', 'Mistral Medium', 'Balanced model', 128000, false),
    ('openrouter', 'mistralai/mistral-small', 'Mistral Small', 'Fast compact model', 128000, false),

    -- Microsoft Phi models
    ('openrouter', 'microsoft/phi-3-medium-128k-instruct', 'Phi-3 Medium 128K', 'Medium model with long context', 128000, false),
    ('openrouter', 'microsoft/phi-3-mini-128k-instruct', 'Phi-3 Mini 128K', 'Compact model with long context', 128000, false),

    -- Qwen models
    ('openrouter', 'qwen/qwen-2.5-72b-instruct', 'Qwen 2.5 72B', 'Alibaba''s powerful model', 128000, false),
    ('openrouter', 'qwen/qwen-2.5-7b-instruct', 'Qwen 2.5 7B', 'Fast small model', 128000, false),

    -- X.AI Grok models
    ('openrouter', 'x-ai/grok-beta', 'Grok Beta', 'X.AI''s Grok model', 128000, false)
ON CONFLICT (provider, model_name) DO NOTHING;

-- Add comment for documentation
COMMENT ON TABLE ai_models IS 'Stores all available AI models from all providers with global enable/disable control. Populated with popular models from OpenAI and OpenRouter.';
