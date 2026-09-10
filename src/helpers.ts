/**
 * Helper function to clean the mnemonic of invalid characters and extraneous spacing
 * @param mnemonic the mnemonic to clean
 * @returns the cleaned mnemonic
 */
export const cleanMnemonic = (mnemonic: String): string => {
  const punctuationRemoved = mnemonic.replace(/[.,\/#!$%\^&\*;:{}=\-_`~()]/g, " ");
  const singleSpace = punctuationRemoved.replace(/\s\s+/g, " ");
  const trimedMnemonic = singleSpace.trim();

  return trimedMnemonic;
};

/**
 * Parses a form field that must hold a non-negative integer (a validator index, an epoch).
 * @returns the number, or `null` if the text is empty or not a plain non-negative integer
 */
export const parseNonNegativeInt = (value: string): number | null => {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  const num = Number(trimmed);
  return Number.isSafeInteger(num) ? num : null;
};

/**
 * Trims a typed amount to the 1 gwei precision the deposit contract works with.
 */
export const trimAmountPrecision = (value: string): string => {
  const [whole, fraction] = value.split(".");
  return fraction === undefined ? whole : `${whole}.${fraction.substring(0, 9)}`;
};
