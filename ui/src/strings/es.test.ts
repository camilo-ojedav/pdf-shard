import { describe, expect, it } from "vitest";
import { S } from "./es";

describe("catálogo de strings (es)", () => {
  it("formatea el indicador de página", () => {
    expect(S.visor.paginaDe(2, 10)).toBe("Página 2 de 10");
  });

  it("formatea el zoom como porcentaje", () => {
    expect(S.visor.zoomPorciento(1.25)).toBe("125%");
  });

  it("no tiene textos vacíos", () => {
    expect(S.app.titulo.length).toBeGreaterThan(0);
    expect(S.visor.abrir.length).toBeGreaterThan(0);
  });
});
