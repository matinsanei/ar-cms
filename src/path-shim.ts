export const sep = '/';
export const delimiter = ':';
export const resolve = (...args: string[]) => args.join('/').replace(/\/+/g, '/');
export const normalize = (p: string) => p;
export const isAbsolute = (p: string) => p.startsWith('/');
export const join = (...args: string[]) => args.join('/').replace(/\/+/g, '/');
export const relative = (_from: string, to: string) => to;
export const dirname = (p: string) => p.split('/').slice(0, -1).join('/') || '/';
export const basename = (p: string, ext?: string) => {
  const b = p.split('/').pop() || '';
  return ext && b.endsWith(ext) ? b.slice(0, -ext.length) : b;
};
export const extname = (p: string) => {
  const b = basename(p);
  const idx = b.lastIndexOf('.');
  return idx > 0 ? b.slice(idx) : '';
};

export default {
  sep,
  delimiter,
  resolve,
  normalize,
  isAbsolute,
  join,
  relative,
  dirname,
  basename,
  extname,
};
