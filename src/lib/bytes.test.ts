import { describe, expect, it } from "vitest";
import { asUint8Array } from "./bytes";

describe("asUint8Array", () => {
  it("accepts Uint8Array and number arrays", () => {
    expect(asUint8Array(new Uint8Array([1, 2, 3]))).toEqual(new Uint8Array([1, 2, 3]));
    expect(asUint8Array([9, 8])).toEqual(new Uint8Array([9, 8]));
    expect(asUint8Array(new Uint8Array([4, 5]).buffer)).toEqual(new Uint8Array([4, 5]));
  });
});
