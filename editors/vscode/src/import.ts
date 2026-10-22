export class ImportValidationError extends Error {}
export interface ImportRpc {
  request(
    method: string,
    params: object,
    signal?: AbortSignal,
    timeout?: number,
  ): Promise<any>;
}
export interface ImportRequest {
  infobase: string;
  sourceDir: string;
  files?: string[];
  objects?: string[];
  changed?: boolean;
}
export interface ImportUi {
  current(): Promise<boolean>;
  confirm(message: string): Promise<boolean>;
}
const kinds: Record<string, string> = {
  Catalogs: "Catalog",
  Documents: "Document",
  CommonModules: "CommonModule",
  CommonForms: "CommonForm",
  InformationRegisters: "InformationRegister",
  Enums: "Enum",
  Reports: "Report",
  DataProcessors: "DataProcessor",
  DefinedTypes: "DefinedType",
  Constants: "Constant",
  Subsystems: "Subsystem",
  ExchangePlans: "ExchangePlan",
  Roles: "Role",
};
function owner(path: string): string {
  const [collection, file] = path.replace(/\\/g, "/").split("/");
  if (!kinds[collection] || !file)
    throw new ImportValidationError(`Cannot add dependencies for ${path}`);
  return `${kinds[collection]}.${file.replace(/\.xml$/, "")}`;
}
async function check(ui: ImportUi, signal?: AbortSignal) {
  if (signal?.aborted)
    throw new ImportValidationError("Import cancelled before publication");
  if (!(await ui.current()))
    throw new ImportValidationError(
      "Connection or settings changed; prepare the import again",
    );
}
export async function runImport(
  rpc: ImportRpc,
  request: ImportRequest,
  ui: ImportUi,
  signal?: AbortSignal,
): Promise<any | undefined> {
  await check(ui, signal);
  let prepared = await rpc.request(
    "objects/import/prepare",
    request,
    signal,
    600000,
  );
  await check(ui, signal);
  const missing = prepared.preflight.missingReferences as {
    reference: string;
    suggestedObject?: string;
  }[];
  if (missing.length) {
    const suggestions = [
      ...new Set(
        missing.map((m) => m.suggestedObject).filter((v): v is string => !!v),
      ),
    ];
    const refs = missing.map((m) => m.reference).join(", ");
    if (!suggestions.length)
      throw new ImportValidationError(`Unresolved references: ${refs}`);
    if (
      !(await ui.confirm(
        `Unresolved references: ${refs}. Add ${suggestions.join(", ")} and recheck? Selected files will include their complete objects. New objects may be unsupported.`,
      ))
    )
      return;
    await check(ui, signal);
    request = {
      ...request,
      files: undefined,
      changed: false,
      objects: [
        ...new Set([
          ...(request.objects ?? []),
          ...(
            request.files ?? (request.changed ? prepared.preflight.files : [])
          ).map(owner),
          ...suggestions,
        ]),
      ],
    };
    prepared = await rpc.request(
      "objects/import/prepare",
      request,
      signal,
      600000,
    );
    await check(ui, signal);
  }
  if (!prepared.preflight.ready)
    throw new ImportValidationError(
      `Unresolved references: ${prepared.preflight.missingReferences.map((m: any) => m.reference).join(", ")}`,
    );
  const files: string[] = prepared.preflight.files;
  if (
    !(await ui.confirm(
      `Загрузить в базу «${request.infobase}» из ${request.sourceDir}: ${files.length} файлов?\n${files.slice(0, 12).join("\n")}\nИзменения останутся в CONFIGSAVE до применения.`,
    ))
  )
    return;
  await check(ui, signal);
  return rpc.request(
    "objects/import",
    {
      infobase: request.infobase,
      sourceDir: request.sourceDir,
      plan: prepared.plan,
      confirm: true,
    },
    signal,
    600000,
  );
}
