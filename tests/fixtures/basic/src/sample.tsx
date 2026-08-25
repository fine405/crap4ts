export function classify(value: number): string {
  if (value > 10 && value < 20) {
    return "middle";
  }
  return value > 20 ? "high" : "low";
}

export const render = (ready: boolean) => ready ? <div /> : null;

export class Picker {
  choose(value: number): string {
    switch (value) {
      case 1:
        return "one";
      case 2:
        return "two";
      default:
        return "other";
    }
  }
}
