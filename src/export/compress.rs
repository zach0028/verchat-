use crate::model::{Conversation, Message, Role};

/// Résultat de l'analyse d'une conversation avant le launch.
#[derive(Debug)]
pub struct ConversationAnalysis {
    /// Nombre total de messages.
    pub total_messages: usize,
    /// Messages user.
    pub user_messages: usize,
    /// Messages assistant (texte).
    pub assistant_messages: usize,
    /// Tokens estimés du dialogue pur (user + assistant texte).
    pub dialogue_tokens: usize,
    /// Tokens estimés des tool calls / thinking (exclus du dialogue).
    pub noise_tokens: usize,
}

/// Analyse une conversation pour estimer la taille du dialogue pur.
pub fn analyze(conv: &Conversation) -> ConversationAnalysis {
    let mut user_messages = 0;
    let mut assistant_messages = 0;
    let mut dialogue_tokens = 0;
    let mut noise_tokens = 0;

    for msg in &conv.messages {
        let tokens = estimate_tokens(&msg.content);
        match msg.role {
            Role::User => {
                user_messages += 1;
                dialogue_tokens += tokens;
            }
            Role::Assistant => {
                assistant_messages += 1;
                dialogue_tokens += tokens;
            }
            Role::System => {
                noise_tokens += tokens;
            }
        }
    }

    ConversationAnalysis {
        total_messages: conv.messages.len(),
        user_messages,
        assistant_messages,
        dialogue_tokens,
        noise_tokens,
    }
}

/// Compresse une conversation pour qu'elle rentre dans un budget de tokens.
///
/// Stratégie : garder le début + la fin, supprimer le milieu.
/// Retourne un seul message `user` formaté avec le contexte.
pub fn compress(conv: &Conversation, max_tokens: usize) -> String {
    let dialogue: Vec<&Message> = conv
        .messages
        .iter()
        .filter(|m| m.role == Role::User || m.role == Role::Assistant)
        .collect();

    if dialogue.is_empty() {
        return String::new();
    }

    let total_tokens: usize = dialogue.iter().map(|m| estimate_tokens(&m.content)).sum();

    // Si ça rentre, pas de compression
    if total_tokens <= max_tokens {
        return format_full(conv, &dialogue);
    }

    // Budget : réserver des tokens pour le header/footer du message
    let overhead = 200; // tokens pour le formatage
    let available = max_tokens.saturating_sub(overhead);

    // Répartir : 30% début, 70% fin (la fin est souvent plus importante)
    let budget_start = available * 30 / 100;
    let budget_end = available * 70 / 100;

    // Sélectionner les messages du début
    let mut start_messages: Vec<&Message> = Vec::new();
    let mut start_tokens = 0;
    for msg in &dialogue {
        let t = estimate_tokens(&msg.content);
        if start_tokens + t > budget_start {
            break;
        }
        start_messages.push(msg);
        start_tokens += t;
    }

    // Sélectionner les messages de la fin (en partant de la fin)
    let mut end_messages: Vec<&Message> = Vec::new();
    let mut end_tokens = 0;
    for msg in dialogue.iter().rev() {
        // Ne pas inclure les messages déjà dans start
        if start_messages.iter().any(|s| std::ptr::eq(*s, *msg)) {
            continue;
        }
        let t = estimate_tokens(&msg.content);
        if end_tokens + t > budget_end {
            break;
        }
        end_messages.push(msg);
        end_tokens += t;
    }
    end_messages.reverse();

    let skipped = dialogue.len() - start_messages.len() - end_messages.len();

    format_compressed(conv, &start_messages, &end_messages, skipped, total_tokens, start_tokens + end_tokens)
}

/// Formate la conversation complète (pas de compression nécessaire).
fn format_full(conv: &Conversation, dialogue: &[&Message]) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Voici le contexte d'une conversation précédente ({}, {}).\n\n",
        conv.source,
        conv.created_at.format("%d/%m/%Y"),
    ));

    for msg in dialogue {
        let role = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };
        output.push_str(&format!("{role}: {}\n\n", msg.content));
    }

    output.push_str("Continue cette conversation à partir de ce contexte.");
    output
}

/// Formate la conversation compressée (début + fin).
fn format_compressed(
    conv: &Conversation,
    start: &[&Message],
    end: &[&Message],
    skipped: usize,
    original_tokens: usize,
    kept_tokens: usize,
) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "Voici le contexte d'une conversation précédente ({}, {}).\n",
        conv.source,
        conv.created_at.format("%d/%m/%Y"),
    ));
    output.push_str(&format!(
        "Note : la conversation originale faisait ~{} tokens. {} messages intermédiaires ont été retirés pour s'adapter à ta fenêtre de contexte (~{} tokens conservés).\n\n",
        format_tokens_human(original_tokens),
        skipped,
        format_tokens_human(kept_tokens),
    ));

    output.push_str("--- DÉBUT DE LA CONVERSATION ---\n\n");

    for msg in start {
        let role = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };
        output.push_str(&format!("{role}: {}\n\n", msg.content));
    }

    output.push_str(&format!("[... {} messages retirés ...]\n\n", skipped));

    output.push_str("--- FIN DE LA CONVERSATION (messages récents) ---\n\n");

    for msg in end {
        let role = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
        };
        output.push_str(&format!("{role}: {}\n\n", msg.content));
    }

    output.push_str("Continue cette conversation à partir de ce contexte.");
    output
}

/// Estime le nombre de tokens d'un texte (~1 token / 4 caractères).
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

fn format_tokens_human(tokens: usize) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{tokens}")
    }
}
