use crate::analyzer::{analyze_named_source, ImportInfo, ModuleInfo, SymbolInfo, SymbolKind};
use crate::formatter::format_named_source;
use crate::interpreter::DgmValue;
use lsp_server::{
    Connection, ErrorCode as LspErrorCode, Message, Notification, Request, RequestId, Response,
    ResponseError,
};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{
    Completion, DocumentSymbolRequest, Formatting, GotoDefinition, HoverRequest,
    References, Rename, Request as _,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentFormattingParams, DocumentSymbol, DocumentSymbolResponse,
    GotoDefinitionResponse, Hover, HoverContents, HoverProviderCapability, InitializeParams,
    Location, MarkedString, OneOf, Position, PublishDiagnosticsParams, Range, ReferenceParams,
    RenameParams, ServerCapabilities, SymbolKind as LspSymbolKind, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, Uri, WorkspaceEdit,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use url::Url as ExternalUrl;

pub fn run_stdio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (connection, _io_threads) = Connection::stdio();
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let init_params = connection.initialize(serde_json::to_value(&server_capabilities)?)?;
    let params: InitializeParams = serde_json::from_value(init_params)?;
    let mut server = LspServer::new(params);
    server.run(&connection)?;
    Ok(())
}

#[derive(Default)]
struct LspServer {
    documents: HashMap<Uri, String>,
    workspace_roots: Vec<PathBuf>,
}

impl LspServer {
    fn new(params: InitializeParams) -> Self {
        let mut workspace_roots = vec![];
        if let Some(folders) = params.workspace_folders {
            for folder in folders {
                if let Some(path) = path_for_uri(&folder.uri) {
                    workspace_roots.push(PathBuf::from(path));
                }
            }
        }
        #[allow(deprecated)]
        if workspace_roots.is_empty() {
            if let Some(root_uri) = params.root_uri {
                if let Some(path) = path_for_uri(&root_uri) {
                    workspace_roots.push(PathBuf::from(path));
                }
            }
        }
        Self {
            documents: HashMap::new(),
            workspace_roots,
        }
    }

    fn run(
        &mut self,
        connection: &Connection,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for message in &connection.receiver {
            match message {
                Message::Request(request) => {
                    if connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    self.handle_request(connection, request.clone())?;
                }
                Message::Notification(notification) => {
                    self.handle_notification(connection, notification.clone())?;
                }
                Message::Response(_) => {}
            }
        }
        Ok(())
    }

    fn handle_notification(
        &mut self,
        connection: &Connection,
        notification: Notification,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)?;
                self.documents
                    .insert(params.text_document.uri.clone(), params.text_document.text);
                self.publish_diagnostics(connection, &params.text_document.uri)?;
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams =
                    serde_json::from_value(notification.params)?;
                if let Some(change) = params.content_changes.into_iter().last() {
                    self.documents
                        .insert(params.text_document.uri.clone(), change.text);
                    self.publish_diagnostics(connection, &params.text_document.uri)?;
                }
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params)?;
                self.documents.remove(&params.text_document.uri);
                self.send_notification(
                    connection,
                    PublishDiagnostics::METHOD,
                    PublishDiagnosticsParams::new(params.text_document.uri, vec![], None),
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_request(
        &mut self,
        connection: &Connection,
        request: Request,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match request.method.as_str() {
            HoverRequest::METHOD => {
                let params = parse_params::<lsp_types::HoverParams>(request.params)?;
                let result = self.handle_hover(params)?;
                self.respond(connection, request.id, result)?;
            }
            GotoDefinition::METHOD => {
                let params = parse_params::<lsp_types::GotoDefinitionParams>(request.params)?;
                let result = self.handle_definition(params)?;
                self.respond(connection, request.id, result)?;
            }
            Completion::METHOD => {
                let params = parse_params::<CompletionParams>(request.params)?;
                let result = self.handle_completion(params)?;
                self.respond(connection, request.id, Some(result))?;
            }
            References::METHOD => {
                let params = parse_params::<ReferenceParams>(request.params)?;
                let result = self.handle_references(params)?;
                self.respond(connection, request.id, Some(result))?;
            }
            Rename::METHOD => {
                let params = parse_params::<RenameParams>(request.params)?;
                let result = self.handle_rename(params)?;
                match result {
                    Ok(edit) => self.respond(connection, request.id, Some(edit))?,
                    Err(error) => self.respond_error(connection, request.id, error)?,
                }
            }
            DocumentSymbolRequest::METHOD => {
                let params =
                    parse_params::<lsp_types::DocumentSymbolParams>(request.params)?;
                let result = self.handle_document_symbols(params)?;
                self.respond(connection, request.id, Some(result))?;
            }
            Formatting::METHOD => {
                let params = parse_params::<DocumentFormattingParams>(request.params)?;
                let result = self.handle_formatting(params)?;
                self.respond(connection, request.id, result)?;
            }
            _ => {
                self.respond_error(
                    connection,
                    request.id,
                    ResponseError {
                        code: LspErrorCode::MethodNotFound as i32,
                        message: format!("unsupported method '{}'", request.method),
                        data: None,
                    },
                )?;
            }
        }
        Ok(())
    }

    fn handle_hover(
        &self,
        params: lsp_types::HoverParams,
    ) -> Result<Option<Hover>, Box<dyn std::error::Error + Send + Sync>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let source = self.source_for_uri(&uri)?;
        let path = path_for_uri(&uri).unwrap_or_else(|| uri.to_string());
        let analysis = analyze_named_source(&source, path.clone())?;

        if let Some(member) = member_access_at_position(&source, position) {
            if let Some(import) = import_for_alias(&analysis.modules, &path, &member.object) {
                if let Some(symbol) = resolve_import_member(&analysis.modules, import, &member.member)
                {
                    if let Some(doc) = doc_for_member(import, &symbol.name) {
                        return Ok(Some(Hover {
                            contents: HoverContents::Scalar(MarkedString::String(doc)),
                            range: Some(member.range),
                        }));
                    }
                    return Ok(Some(symbol_hover(&symbol, member.range)));
                }
            }
        }

        if let Some(symbol) = symbol_target_at_position(&analysis, &path, position) {
            if let Some(doc) = doc_for_symbol(&symbol.name) {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(doc)),
                    range: Some(range_for_named_span(&symbol.span, &symbol.name)),
                }));
            }
            return Ok(Some(symbol_hover(
                &symbol,
                range_for_named_span(&symbol.span, &symbol.name),
            )));
        }

        Ok(None)
    }

    fn handle_definition(
        &self,
        params: lsp_types::GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>, Box<dyn std::error::Error + Send + Sync>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let source = self.source_for_uri(&uri)?;
        let path = path_for_uri(&uri).unwrap_or_else(|| uri.to_string());
        let analysis = analyze_named_source(&source, path.clone())?;

        if let Some(member) = member_access_at_position(&source, position) {
            if let Some(import) = import_for_alias(&analysis.modules, &path, &member.object) {
                if let Some(symbol) = resolve_import_member(&analysis.modules, import, &member.member)
                {
                    if let Some(location) = location_for_symbol(&symbol) {
                        return Ok(Some(GotoDefinitionResponse::Scalar(location)));
                    }
                }
            }
        }

        if let Some(symbol) = symbol_target_at_position(&analysis, &path, position) {
            if let Some(location) = location_for_symbol(&symbol) {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        Ok(None)
    }

    fn handle_completion(
        &self,
        params: CompletionParams,
    ) -> Result<CompletionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let source = self.source_for_uri(&uri)?;
        let path = path_for_uri(&uri).unwrap_or_else(|| uri.to_string());
        let analysis = analyze_named_source(&source, path.clone())?;

        if let Some(member) = member_completion_context(&source, position) {
            if let Some(import) = import_for_alias(&analysis.modules, &path, &member.object) {
                let mut items = module_completion_items(&analysis.modules, import);
                items.retain(|item| item.label.starts_with(&member.partial));
                return Ok(CompletionResponse::Array(items));
            }
        }

        let prefix = identifier_prefix(&source, position);
        let mut seen = HashSet::new();
        let mut items = vec![];
        for keyword in KEYWORDS {
            push_completion(
                &mut items,
                &mut seen,
                keyword,
                CompletionItemKind::KEYWORD,
                None,
            );
        }
        for builtin in BUILTINS {
            push_completion(
                &mut items,
                &mut seen,
                builtin,
                CompletionItemKind::FUNCTION,
                doc_for_symbol(builtin),
            );
        }
        for symbol in analysis.symbols.iter().filter(|symbol| symbol.span.file.as_ref() == &path) {
            push_completion(
                &mut items,
                &mut seen,
                &symbol.name,
                completion_kind_for_symbol(symbol.kind.clone()),
                None,
            );
        }
        items.retain(|item| prefix.is_empty() || item.label.starts_with(&prefix));
        Ok(CompletionResponse::Array(items))
    }

    fn handle_references(
        &self,
        params: ReferenceParams,
    ) -> Result<Vec<Location>, Box<dyn std::error::Error + Send + Sync>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let source = self.source_for_uri(&uri)?;
        let path = path_for_uri(&uri).unwrap_or_else(|| uri.to_string());
        let analysis = analyze_named_source(&source, path.clone())?;

        let Some(target) = self.resolve_target_symbol(&analysis, &source, &path, position) else {
            return Ok(vec![]);
        };

        let mut locations = collect_symbol_locations(&analysis, &target);
        locations.extend(self.collect_import_member_locations(&analysis.modules, &target)?);
        dedupe_locations(&mut locations);

        if !params.context.include_declaration {
            let definition = location_for_symbol(&target);
            if let Some(definition) = definition {
                locations.retain(|location| location != &definition);
            }
        }

        Ok(locations)
    }

    fn handle_rename(
        &self,
        params: RenameParams,
    ) -> Result<Result<WorkspaceEdit, ResponseError>, Box<dyn std::error::Error + Send + Sync>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let source = self.source_for_uri(&uri)?;
        let path = path_for_uri(&uri).unwrap_or_else(|| uri.to_string());
        let analysis = analyze_named_source(&source, path.clone())?;

        let Some(target) = self.resolve_target_symbol(&analysis, &source, &path, position) else {
            return Ok(Err(response_error(
                LspErrorCode::InvalidParams,
                "no rename target at cursor",
            )));
        };

        if target.span.file.starts_with('<') {
            return Ok(Err(response_error(
                LspErrorCode::InvalidParams,
                "cannot rename builtin or stdlib symbol",
            )));
        }

        let mut edits_by_uri: HashMap<Uri, Vec<TextEdit>> = HashMap::new();
        for location in collect_symbol_locations(&analysis, &target) {
            edits_by_uri.entry(location.uri).or_default().push(TextEdit {
                range: location.range,
                new_text: params.new_name.clone(),
            });
        }
        for location in self.collect_import_member_locations(&analysis.modules, &target)? {
            edits_by_uri.entry(location.uri).or_default().push(TextEdit {
                range: location.range,
                new_text: params.new_name.clone(),
            });
        }

        Ok(Ok(WorkspaceEdit {
            changes: Some(edits_by_uri),
            ..Default::default()
        }))
    }

    fn handle_document_symbols(
        &self,
        params: lsp_types::DocumentSymbolParams,
    ) -> Result<DocumentSymbolResponse, Box<dyn std::error::Error + Send + Sync>> {
        let uri = params.text_document.uri;
        let source = self.source_for_uri(&uri)?;
        let path = path_for_uri(&uri).unwrap_or_else(|| uri.to_string());
        let analysis = analyze_named_source(&source, path.clone())?;

        let exports = analysis
            .modules
            .get(&path)
            .map(|module| module.exports.clone())
            .unwrap_or_default();
        #[allow(deprecated)]
        let symbols = exports
            .into_iter()
            .map(|symbol| DocumentSymbol {
                name: symbol.name.clone(),
                detail: Some(symbol_detail(&symbol)),
                kind: lsp_symbol_kind(symbol.kind.clone()),
                tags: None,
                deprecated: None,
                range: range_for_named_span(&symbol.span, &symbol.name),
                selection_range: range_for_named_span(&symbol.span, &symbol.name),
                children: None,
            })
            .collect();

        Ok(DocumentSymbolResponse::Nested(symbols))
    }

    fn handle_formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>, Box<dyn std::error::Error + Send + Sync>> {
        let uri = params.text_document.uri;
        let source = self.source_for_uri(&uri)?;
        let path = path_for_uri(&uri).unwrap_or_else(|| uri.to_string());
        let formatted = format_named_source(&source, path)?;
        if formatted == source {
            return Ok(Some(vec![]));
        }
        let last_line = source.lines().count().max(1) as u32;
        let last_col = source
            .lines()
            .last()
            .map(|line| line.len() as u32)
            .unwrap_or(0);
        Ok(Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(last_line, last_col)),
            new_text: formatted,
        }]))
    }

    fn publish_diagnostics(
        &self,
        connection: &Connection,
        uri: &Uri,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source = self.source_for_uri(uri)?;
        let path = path_for_uri(uri).unwrap_or_else(|| uri.to_string());
        let diagnostics = match analyze_named_source(&source, path) {
            Ok(result) => result
                .diagnostics
                .into_iter()
                .map(|error| diagnostic_from_error(&error))
                .collect(),
            Err(error) => vec![diagnostic_from_error(&error)],
        };

        self.send_notification(
            connection,
            PublishDiagnostics::METHOD,
            PublishDiagnosticsParams::new(uri.clone(), diagnostics, None),
        )
    }

    fn send_notification<P: serde::Serialize>(
        &self,
        connection: &Connection,
        method: &'static str,
        params: P,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        connection.sender.send(Message::Notification(Notification::new(
            method.to_string(),
            serde_json::to_value(params)?,
        )))?;
        Ok(())
    }

    fn respond<R: serde::Serialize>(
        &self,
        connection: &Connection,
        id: RequestId,
        result: R,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        connection.sender.send(Message::Response(Response {
            id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        }))?;
        Ok(())
    }

    fn respond_error(
        &self,
        connection: &Connection,
        id: RequestId,
        error: ResponseError,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        connection.sender.send(Message::Response(Response {
            id,
            result: None,
            error: Some(error),
        }))?;
        Ok(())
    }

    fn source_for_uri(
        &self,
        uri: &Uri,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(source) = self.documents.get(uri) {
            return Ok(source.clone());
        }
        let path = path_for_uri(uri)
            .ok_or_else(|| format!("URI is not a file path: {}", uri.as_str()))?;
        Ok(fs::read_to_string(path)?)
    }

    fn source_for_path(
        &self,
        path: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        for (uri, source) in &self.documents {
            if path_for_uri(uri).as_deref() == Some(path) {
                return Ok(source.clone());
            }
        }
        Ok(fs::read_to_string(path)?)
    }

    fn resolve_target_symbol(
        &self,
        analysis: &crate::analyzer::AnalysisResult,
        source: &str,
        path: &str,
        position: Position,
    ) -> Option<SymbolInfo> {
        if let Some(member) = member_access_at_position(source, position) {
            if let Some(import) = import_for_alias(&analysis.modules, path, &member.object) {
                if let Some(symbol) = resolve_import_member(&analysis.modules, import, &member.member)
                {
                    return Some(symbol);
                }
            }
        }
        symbol_target_at_position(analysis, path, position)
    }

    fn collect_import_member_locations(
        &self,
        modules: &HashMap<String, ModuleInfo>,
        target: &SymbolInfo,
    ) -> Result<Vec<Location>, Box<dyn std::error::Error + Send + Sync>> {
        let target_file = target.span.file.as_ref();
        let mut locations = vec![];
        let mut candidate_files: HashSet<String> = modules.keys().cloned().collect();
        candidate_files.extend(self.collect_workspace_files()?);
        for file in candidate_files {
            let source = self.source_for_path(&file)?;
            let analysis = match analyze_named_source(&source, file.clone()) {
                Ok(analysis) => analysis,
                Err(_) => continue,
            };
            let Some(module) = analysis.modules.get(&file) else {
                continue;
            };
            let aliases: HashSet<_> = module
                .imports
                .iter()
                .filter(|import| import.resolved.as_deref() == Some(target_file))
                .map(|import| import.alias.clone())
                .collect();
            if aliases.is_empty() {
                continue;
            }
            for (alias, member_range) in
                scan_import_member_references(&source, &file, &aliases, &target.name)
            {
                if aliases.contains(&alias) {
                    if let Some(uri) = uri_from_file_path(&file) {
                        locations.push(Location::new(uri, member_range));
                    }
                }
            }
        }
        Ok(locations)
    }

    fn collect_workspace_files(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut files = vec![];
        for root in &self.workspace_roots {
            collect_dgm_files(root, &mut files)?;
        }
        Ok(files)
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(
    value: Value,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
    Ok(serde_json::from_value(value)?)
}

fn response_error(code: LspErrorCode, message: impl Into<String>) -> ResponseError {
    ResponseError {
        code: code as i32,
        message: message.into(),
        data: None,
    }
}

fn diagnostic_from_error(error: &crate::error::DgmError) -> Diagnostic {
    Diagnostic {
        range: error
            .span
            .as_ref()
            .map(|span| range_for_named_span(span, "x"))
            .unwrap_or_else(|| Range::new(Position::new(0, 0), Position::new(0, 1))),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(lsp_types::NumberOrString::String(error.code.as_str().to_string())),
        code_description: None,
        source: Some("dgm".to_string()),
        message: error.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

fn symbol_target_at_position(
    analysis: &crate::analyzer::AnalysisResult,
    path: &str,
    position: Position,
) -> Option<SymbolInfo> {
    if let Some(reference) = analysis.references.iter().find(|reference| {
        reference.span.file.as_ref() == path
            && position_in_named_span(position, &reference.span, &reference.name)
    }) {
        return reference.target.clone();
    }

    analysis
        .symbols
        .iter()
        .find(|symbol| {
            symbol.span.file.as_ref() == path
                && position_in_named_span(position, &symbol.span, &symbol.name)
        })
        .cloned()
}

fn position_in_named_span(position: Position, span: &crate::Span, name: &str) -> bool {
    if position.line != span.line.saturating_sub(1) as u32 {
        return false;
    }
    let start = span.col.saturating_sub(1) as u32;
    let end = start + name.len() as u32;
    position.character >= start && position.character <= end
}

fn range_for_named_span(span: &crate::Span, name: &str) -> Range {
    let start = Position::new(span.line.saturating_sub(1) as u32, span.col.saturating_sub(1) as u32);
    let end = Position::new(start.line, start.character + name.len() as u32);
    Range::new(start, end)
}

fn location_for_symbol(symbol: &SymbolInfo) -> Option<Location> {
    if symbol.span.file.starts_with('<') {
        return None;
    }
    let uri = uri_from_file_path(symbol.span.file.as_ref())?;
    Some(Location::new(uri, range_for_named_span(&symbol.span, &symbol.name)))
}

fn collect_symbol_locations(
    analysis: &crate::analyzer::AnalysisResult,
    target: &SymbolInfo,
) -> Vec<Location> {
    let mut locations = vec![];
    if let Some(location) = location_for_symbol(target) {
        locations.push(location);
    }
    for reference in &analysis.references {
        if let Some(symbol) = &reference.target {
            if same_symbol(symbol, target) {
                if let Some(uri) = uri_from_file_path(reference.span.file.as_ref()) {
                    locations.push(Location::new(
                        uri,
                        range_for_named_span(&reference.span, &reference.name),
                    ));
                }
            }
        }
    }
    locations
}

fn same_symbol(left: &SymbolInfo, right: &SymbolInfo) -> bool {
    left.name == right.name
        && left.kind == right.kind
        && left.span.file == right.span.file
        && left.span.line == right.span.line
        && left.span.col == right.span.col
}

fn dedupe_locations(locations: &mut Vec<Location>) {
    let mut seen = HashSet::new();
    locations.retain(|location| {
        seen.insert((
            location.uri.clone(),
            location.range.start.line,
            location.range.start.character,
            location.range.end.line,
            location.range.end.character,
        ))
    });
}

fn import_for_alias<'a>(
    modules: &'a HashMap<String, ModuleInfo>,
    file: &str,
    alias: &str,
) -> Option<&'a ImportInfo> {
    modules.get(file)?.imports.iter().find(|import| import.alias == alias)
}

fn resolve_import_member(
    modules: &HashMap<String, ModuleInfo>,
    import: &ImportInfo,
    member: &str,
) -> Option<SymbolInfo> {
    if import.is_stdlib {
        return stdlib_exports(&import.module)
            .into_iter()
            .find(|symbol| symbol.name == member);
    }

    let resolved = import.resolved.as_ref()?;
    modules
        .get(resolved)?
        .exports
        .iter()
        .find(|symbol| symbol.name == member)
        .cloned()
}

fn stdlib_exports(module: &str) -> Vec<SymbolInfo> {
    match crate::stdlib::load_module(module) {
        Some(DgmValue::Map(map)) => {
            let mut names: Vec<_> = map.borrow().keys().cloned().collect();
            names.sort();
            names
                .into_iter()
                .map(|name| SymbolInfo {
                    name,
                    kind: SymbolKind::Function,
                    span: crate::Span::new(("<stdlib>".to_string()).into(), 1, 1),
                })
                .collect()
        }
        _ => vec![],
    }
}

fn module_completion_items(
    modules: &HashMap<String, ModuleInfo>,
    import: &ImportInfo,
) -> Vec<CompletionItem> {
    let symbols = if import.is_stdlib {
        stdlib_exports(&import.module)
    } else {
        import
            .resolved
            .as_ref()
            .and_then(|resolved| modules.get(resolved))
            .map(|module| module.exports.clone())
            .unwrap_or_default()
    };
    symbols
        .into_iter()
        .map(|symbol| {
            let detail = symbol_detail(&symbol);
            CompletionItem {
                label: symbol.name.clone(),
                kind: Some(completion_kind_for_symbol(symbol.kind.clone())),
                detail: Some(detail),
                documentation: doc_for_member(import, &symbol.name)
                    .map(lsp_types::Documentation::String),
                ..Default::default()
            }
        })
        .collect()
}

fn symbol_detail(symbol: &SymbolInfo) -> String {
    match &symbol.kind {
        SymbolKind::Function => format!("fn {}", symbol.name),
        SymbolKind::Class => format!("class {}", symbol.name),
        SymbolKind::Import => format!("import {}", symbol.name),
        SymbolKind::Constant => format!("const {}", symbol.name),
        SymbolKind::Parameter => format!("param {}", symbol.name),
        SymbolKind::CatchVar => format!("catch {}", symbol.name),
        SymbolKind::Variable => format!("let {}", symbol.name),
    }
}

fn symbol_hover(symbol: &SymbolInfo, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Scalar(MarkedString::LanguageString(lsp_types::LanguageString {
            language: "dgm".to_string(),
            value: symbol_detail(&symbol),
        })),
        range: Some(range),
    }
}

fn completion_kind_for_symbol(kind: SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::Function => CompletionItemKind::FUNCTION,
        SymbolKind::Class => CompletionItemKind::CLASS,
        SymbolKind::Import => CompletionItemKind::MODULE,
        SymbolKind::Constant => CompletionItemKind::CONSTANT,
        SymbolKind::Parameter => CompletionItemKind::VARIABLE,
        SymbolKind::CatchVar | SymbolKind::Variable => CompletionItemKind::VARIABLE,
    }
}

fn lsp_symbol_kind(kind: SymbolKind) -> LspSymbolKind {
    match kind {
        SymbolKind::Function => LspSymbolKind::FUNCTION,
        SymbolKind::Class => LspSymbolKind::CLASS,
        SymbolKind::Import => LspSymbolKind::MODULE,
        SymbolKind::Constant => LspSymbolKind::CONSTANT,
        SymbolKind::Parameter => LspSymbolKind::VARIABLE,
        SymbolKind::CatchVar | SymbolKind::Variable => LspSymbolKind::VARIABLE,
    }
}

fn push_completion(
    items: &mut Vec<CompletionItem>,
    seen: &mut HashSet<String>,
    label: &str,
    kind: CompletionItemKind,
    documentation: Option<String>,
) {
    if !seen.insert(label.to_string()) {
        return;
    }
    items.push(CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        documentation: documentation.map(lsp_types::Documentation::String),
        ..Default::default()
    });
}

fn path_for_uri(uri: &Uri) -> Option<String> {
    ExternalUrl::parse(uri.as_str())
        .ok()?
        .to_file_path()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
}

fn uri_from_file_path(path: &str) -> Option<Uri> {
    let url = ExternalUrl::from_file_path(path).ok()?;
    url.as_str().parse().ok()
}

fn collect_dgm_files(
    root: &PathBuf,
    files: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            collect_dgm_files(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("dgm") {
            files.push(path.to_string_lossy().to_string());
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct MemberAccess {
    object: String,
    member: String,
    range: Range,
}

fn member_access_at_position(source: &str, position: Position) -> Option<MemberAccess> {
    let line = source.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let mut start = position.character as usize;
    if start >= chars.len() && start > 0 {
        start -= 1;
    }
    while start > 0 && is_ident_char(chars[start.saturating_sub(1)]) {
        start -= 1;
    }
    let mut end = position.character as usize;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }
    if start >= end {
        return None;
    }
    let member: String = chars[start..end].iter().collect();
    let mut dot = start;
    while dot > 0 && chars[dot - 1].is_whitespace() {
        dot -= 1;
    }
    if dot == 0 || chars[dot - 1] != '.' {
        return None;
    }
    let mut object_end = dot - 1;
    while object_end > 0 && chars[object_end - 1].is_whitespace() {
        object_end -= 1;
    }
    let mut object_start = object_end;
    while object_start > 0 && is_ident_char(chars[object_start - 1]) {
        object_start -= 1;
    }
    if object_start == object_end {
        return None;
    }
    let object: String = chars[object_start..object_end].iter().collect();
    Some(MemberAccess {
        object,
        member: member.clone(),
        range: Range::new(
            Position::new(position.line, start as u32),
            Position::new(position.line, end as u32),
        ),
    })
}

#[derive(Debug, Clone)]
struct MemberCompletionContext {
    object: String,
    partial: String,
}

fn member_completion_context(source: &str, position: Position) -> Option<MemberCompletionContext> {
    let line = source.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    let mut cursor = position.character as usize;
    if cursor > chars.len() {
        cursor = chars.len();
    }
    let mut member_start = cursor;
    while member_start > 0 && is_ident_char(chars[member_start - 1]) {
        member_start -= 1;
    }
    let partial: String = chars[member_start..cursor].iter().collect();
    if member_start == 0 || chars[member_start - 1] != '.' {
        return None;
    }
    let mut object_end = member_start - 1;
    while object_end > 0 && chars[object_end - 1].is_whitespace() {
        object_end -= 1;
    }
    let mut object_start = object_end;
    while object_start > 0 && is_ident_char(chars[object_start - 1]) {
        object_start -= 1;
    }
    if object_start == object_end {
        return None;
    }
    Some(MemberCompletionContext {
        object: chars[object_start..object_end].iter().collect(),
        partial,
    })
}

fn identifier_prefix(source: &str, position: Position) -> String {
    let Some(line) = source.lines().nth(position.line as usize) else {
        return String::new();
    };
    let chars: Vec<char> = line.chars().collect();
    let mut cursor = position.character as usize;
    if cursor > chars.len() {
        cursor = chars.len();
    }
    let mut start = cursor;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    chars[start..cursor].iter().collect()
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn scan_import_member_references(
    source: &str,
    file: &str,
    aliases: &HashSet<String>,
    member: &str,
) -> Vec<(String, Range)> {
    let mut matches = vec![];
    if let Ok(tokens) = crate::tokenize_named_source(source, file.to_string()) {
        for window in tokens.windows(3) {
            if let [
                crate::DgmToken {
                    kind: crate::TokenKind::Ident,
                    lexeme: alias,
                    ..
                },
                crate::DgmToken {
                    kind: crate::TokenKind::Dot,
                    ..
                },
                crate::DgmToken {
                    kind: crate::TokenKind::Ident,
                    lexeme,
                    line,
                    col,
                    ..
                },
            ] = window
            {
                if aliases.contains(alias) && lexeme == member {
                    let span = crate::Span::new(file.to_string().into(), *line, *col);
                    matches.push((alias.clone(), range_for_named_span(&span, lexeme)));
                }
            }
        }
    }
    matches
}

fn doc_for_symbol(name: &str) -> Option<String> {
    builtin_doc(name).map(|(signature, description)| format!("{}\n\n{}", signature, description))
}

fn doc_for_member(import: &ImportInfo, member: &str) -> Option<String> {
    let key = if import.is_stdlib {
        format!("{}.{}", import.module, member)
    } else {
        member.to_string()
    };
    builtin_doc(&key)
        .or_else(|| builtin_doc(member))
        .map(|(signature, description)| format!("{}\n\n{}", signature, description))
}

fn builtin_doc(name: &str) -> Option<(&'static str, &'static str)> {
    Some(match name {
        "len" => ("len(value)", "Return the length of a list, string, or map."),
        "type" => ("type(value)", "Return the runtime type name for a value."),
        "str" => ("str(value)", "Convert a value to a string."),
        "int" => ("int(value)", "Convert a value to an integer."),
        "float" => ("float(value)", "Convert a value to a float."),
        "range" => ("range(end) | range(start, end)", "Create a list of integers in sequence."),
        "map" => ("map(list, fn)", "Apply a callback to each list item and return a new list."),
        "filter" => ("filter(list, fn)", "Keep items whose callback returns true."),
        "reduce" => ("reduce(list, init, fn)", "Fold a list into a single value."),
        "each" => ("each(list, fn)", "Run a callback for each list item."),
        "find" => ("find(list, fn)", "Return the first matching item."),
        "any" => ("any(list, fn?)", "Return true if any item matches."),
        "all" => ("all(list, fn?)", "Return true if every item matches."),
        "math.sqrt" => ("math.sqrt(n)", "Return the square root of a number."),
        "math.pow" => ("math.pow(base, exp)", "Raise a number to a power."),
        "json.parse" => ("json.parse(string)", "Parse a JSON string into DGM values."),
        "json.stringify" => ("json.stringify(value)", "Serialize a DGM value as JSON."),
        "http.get" => ("http.get(url, opts?)", "Perform an HTTP GET request."),
        "http.post" => ("http.post(url, body, opts?)", "Perform an HTTP POST request."),
        "http.request" => ("http.request(method, url, body?, opts?)", "Perform an HTTP request."),
        "http.serve" => ("http.serve(port, handler)", "Start a simple HTTP server."),
        "regex.match" => ("regex.match(pattern, text)", "Check whether a regex matches text."),
        "regex.find_all" => ("regex.find_all(pattern, text)", "Return all regex matches."),
        "xml.parse" => ("xml.parse(string)", "Parse XML into the DGM XML node map shape."),
        "xml.stringify" => ("xml.stringify(node)", "Serialize a DGM XML node map into XML."),
        "xml.query" => ("xml.query(node, path)", "Find a nested XML child node by dotted path."),
        _ => return None,
    })
}

const KEYWORDS: &[&str] = &[
    "import", "let", "const", "fn", "return", "if", "else", "for", "while", "break",
    "continue", "class", "extends", "new", "this", "super", "try", "catch", "finally",
    "throw", "match", "lam", "true", "false", "null", "and", "or", "in",
];

const BUILTINS: &[&str] = &[
    "len", "type", "str", "int", "float", "push", "pop", "range", "input", "abs", "min",
    "max", "sort", "reverse", "keys", "values", "has_key", "slice", "join", "split",
    "replace", "upper", "lower", "trim", "contains", "starts_with", "ends_with", "chars",
    "format", "index_of", "flat", "zip", "sum", "print", "println", "chr", "ord", "hex",
    "bin", "exit", "assert", "map", "filter", "reduce", "each", "find", "any", "all",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_member_access_under_cursor() {
        let source = "import math as m\nlet value = m.sqrt(9)\n";
        let access = member_access_at_position(source, Position::new(1, 16)).unwrap();
        assert_eq!(access.object, "m");
        assert_eq!(access.member, "sqrt");
    }

    #[test]
    fn detects_member_completion_context_after_dot() {
        let source = "import math as m\nm.sq\n";
        let ctx = member_completion_context(source, Position::new(1, 4)).unwrap();
        assert_eq!(ctx.object, "m");
        assert_eq!(ctx.partial, "sq");
    }

    #[test]
    fn scans_import_member_references_from_tokens() {
        let source = "import helper\nhelper.value()\n";
        let aliases = HashSet::from([String::from("helper")]);
        let refs = scan_import_member_references(source, "/tmp/main.dgm", &aliases, "value");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].0, "helper");
        assert_eq!(refs[0].1.start.line, 1);
    }
}
