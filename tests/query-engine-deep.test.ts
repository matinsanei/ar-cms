import { describe, it, expect, beforeAll } from 'vitest';

const BACKEND_URL = 'http://localhost:8088';

describe('Deep Query Engine, Filters, Search & Geo-Spatial Suite', () => {
  let authToken = '';

  beforeAll(async () => {
    const loginRes = await fetch(`${BACKEND_URL}/admin/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email: 'matin@pixelstack.io', password: 'Password123!' }),
    });
    const loginData = await loginRes.json();
    authToken = loginData.data.token;

    // Seed test products with diverse attributes and coordinates
    const testItems = [
      { name: 'MacBook Pro 16 M3 Max', price: 3499, category: 'Laptops', rating: 5, location: { lat: 35.6892, lng: 51.3890 } }, // Tehran
      { name: 'Dell XPS 15 OLED', price: 2199, category: 'Laptops', rating: 4, location: { lat: 35.7152, lng: 51.4043 } }, // Tehran North
      { name: 'Sony WH-1000XM5 Headphones', price: 399, category: 'Audio', rating: 5, location: { lat: 35.8052, lng: 51.4300 } }, // Tajrish
      { name: 'Apple AirPods Pro 2', price: 249, category: 'Audio', rating: 4, location: { lat: 32.6546, lng: 51.6680 } }, // Isfahan (~340km)
      { name: 'Logitech MX Master 3S', price: 99, category: 'Accessories', rating: 5, location: { lat: 29.5918, lng: 52.5837 } }, // Shiraz (~680km)
    ];

    for (const item of testItems) {
      const createRes = await fetch(`${BACKEND_URL}/content-manager/collection-types/api::product.product`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${authToken}`,
        },
        body: JSON.stringify(item),
      });
      const createData = await createRes.json();
      const docId = createData.data.documentId;

      // Publish so they are accessible via storefront queries
      await fetch(`${BACKEND_URL}/content-manager/collection-types/api::product.product/${docId}/actions/publish`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${authToken}` },
      });
    }
  });

  describe('1. Exact Match ($eq) and Inequality ($ne) Filters', () => {
    it('filters exact match by string and number: filters[category][$eq]=Laptops', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?filters[category][$eq]=Laptops`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBeGreaterThanOrEqual(2);
      for (const item of data.data) {
        expect(item.category).toBe('Laptops');
      }
    });

    it('filters inequality: filters[category][$ne]=Laptops', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?filters[category][$ne]=Laptops`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBeGreaterThanOrEqual(3);
      for (const item of data.data) {
        expect(item.category).not.toBe('Laptops');
      }
    });
  });

  describe('2. String Pattern Filters ($contains, $startsWith, $endsWith)', () => {
    it('$contains performs case-insensitive substring search', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?filters[name][$contains]=macbook`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBeGreaterThanOrEqual(1);
      expect(data.data[0].name).toContain('MacBook Pro');
    });

    it('$startsWith matches string prefix', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?filters[name][$startsWith]=Sony`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBeGreaterThanOrEqual(1);
      expect(data.data[0].name.startsWith('Sony')).toBe(true);
    });

    it('$endsWith matches string suffix', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?filters[name][$endsWith]=Headphones`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBeGreaterThanOrEqual(1);
      expect(data.data[0].name.endsWith('Headphones')).toBe(true);
    });
  });

  describe('3. Full-Text Search (_q) and Multi-condition Compound Filters', () => {
    it('_q search matches keyword across multiple attributes', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?_q=AirPods`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBeGreaterThanOrEqual(1);
      expect(data.data[0].name).toContain('AirPods');
    });

    it('combines multiple filters with logical AND', async () => {
      const res = await fetch(
        `${BACKEND_URL}/api/products?filters[category][$eq]=Laptops&filters[name][$contains]=OLED`
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBeGreaterThanOrEqual(1);
      for (const item of data.data) {
        expect(item.name).toContain('OLED');
        expect(item.category).toBe('Laptops');
      }
    });
  });

  describe('4. Sorting & Pagination Controls', () => {
    it('sorts ascending by price (sort=price:ASC)', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?sort=price:ASC&pageSize=5`);
      expect(res.status).toBe(200);
      const data = await res.json();
      for (let i = 1; i < data.data.length; i++) {
        expect(Number(data.data[i].price)).toBeGreaterThanOrEqual(Number(data.data[i - 1].price));
      }
    });

    it('sorts descending by price (sort=price:DESC)', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?sort=price:DESC&pageSize=5`);
      expect(res.status).toBe(200);
      const data = await res.json();
      for (let i = 1; i < data.data.length; i++) {
        expect(Number(data.data[i].price)).toBeLessThanOrEqual(Number(data.data[i - 1].price));
      }
    });

    it('handles pagination with correct pageCount and total metadata', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?pagination[page]=1&pagination[pageSize]=2`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data.length).toBe(2);
      expect(data.meta.pagination.page).toBe(1);
      expect(data.meta.pagination.pageSize).toBe(2);
      expect(data.meta.pagination.total).toBeGreaterThanOrEqual(5);
      expect(data.meta.pagination.pageCount).toBeGreaterThanOrEqual(3);
    });

    it('handles out-of-range pagination gracefully by returning empty array', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products?pagination[page]=9999&pagination[pageSize]=10`);
      expect(res.status).toBe(200);
      const data = await res.json();
      expect(data.data).toEqual([]);
      expect(data.meta.pagination.page).toBe(9999);
    });
  });

  describe('5. Geo-Spatial Proximity ($near) and RFC 7946 GeoJSON Standard', () => {
    it('filters points within radius using Haversine calculation ($near)', async () => {
      // Query within 20km of Central Tehran (35.6892, 51.3890)
      const res = await fetch(
        `${BACKEND_URL}/api/products?filters[location][$near]=35.6892,51.3890,20000`
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      // Should include Tehran and Tajrish items, but exclude Isfahan (~340km) and Shiraz (~680km)
      const names = data.data.map((p: any) => p.name);
      expect(names).toContain('MacBook Pro 16 M3 Max');
      expect(names).toContain('Dell XPS 15 OLED');
      expect(names).toContain('Sony WH-1000XM5 Headphones');
      expect(names).not.toContain('Apple AirPods Pro 2'); // Isfahan
      expect(names).not.toContain('Logitech MX Master 3S'); // Shiraz
    });

    it('sorts locations by geographical proximity from origin (origin=lat,lng)', async () => {
      // Origin at Central Tehran
      const res = await fetch(
        `${BACKEND_URL}/api/products?origin=35.6892,51.3890&sort=location:asc`
      );
      expect(res.status).toBe(200);
      const data = await res.json();
      // Nearest item must be MacBook Pro (same point: 0km)
      expect(['MacBook Pro 16 M3 Max', 'Tehran Tech Hub']).toContain(data.data[0].name);
    });

    it('GET /api/products/geojson returns standard RFC 7946 FeatureCollection', async () => {
      const res = await fetch(`${BACKEND_URL}/api/products/geojson`);
      expect(res.status).toBe(200);
      expect(res.headers.get('content-type')).toContain('geo+json');
      const geojson = await res.json();

      expect(geojson.type).toBe('FeatureCollection');
      expect(Array.isArray(geojson.features)).toBe(true);
      expect(geojson.features.length).toBeGreaterThanOrEqual(5);

      for (const feat of geojson.features) {
        expect(feat.type).toBe('Feature');
        expect(feat.geometry.type).toBe('Point');
        expect(Array.isArray(feat.geometry.coordinates)).toBe(true);
        expect(feat.geometry.coordinates.length).toBe(2);
        // Longitude first, then Latitude (RFC 7946)
        expect(typeof feat.geometry.coordinates[0]).toBe('number'); // lng
        expect(typeof feat.geometry.coordinates[1]).toBe('number'); // lat
        expect(feat.properties).toBeDefined();
        expect(feat.properties.id).toBeDefined();
        expect(feat.properties.documentId).toBeDefined();
      }
    });

    it('GET /content-manager/collection-types/:uid/geojson preview for admin manager', async () => {
      const res = await fetch(`${BACKEND_URL}/content-manager/collection-types/api::product.product/geojson`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      expect(res.status).toBe(200);
      const geojson = await res.json();
      expect(geojson.type).toBe('FeatureCollection');
    });
  });
});
